// This example demonstrates the use of the appsink element.
// It operates the following pipeline:

// {audiotestsrc} - {appsink}

// The application specifies what format it wants to handle. This format
// is applied by calling set_caps on the appsink. Now it's the audiotestsrc's
// task to provide this data format. If the element connected to the appsink's
// sink-pad were not able to provide what we ask them to, this would fail.
// This is the format we request:
// Audio / Signed 16bit / 1 channel / arbitrary sample rate

use bevy::asset::AssetLoader;
use bevy::asset::LoadContext;
use bevy::asset::LoadedAsset;
use bevy::prelude::Component;
use bevy::prelude::Handle;
use bevy::prelude::Image;
use bevy::reflect::TypeUuid;
use bevy::utils::BoxedFuture;
use gst::element_error;
use gst::prelude::*;

use byte_slice_cast::*;

use std::i16;
use std::i32;
use std::sync::Arc;
use std::sync::RwLock;

use anyhow::Error;
use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
#[display(fmt = "Missing element {}", _0)]
struct MissingElement(#[error(not(source))] &'static str);

#[derive(Debug, Display, Error)]
#[display(fmt = "Received error from {}: {} (debug: {:?})", src, error, debug)]
pub struct ErrorMessage {
    pub src: String,
    pub error: String,
    pub debug: Option<String>,
    pub source: glib::Error,
}

type ImageRaw = [u8; 176 * 144 * 4];

#[derive(Debug, TypeUuid)]
#[uuid = "39cadc56-aa9c-4543-8640-a018b74b5052"]
pub struct AppSinkImage {
    pub pipeline: gst::Pipeline,
    pub bus: gst::Bus,
    pub image_raw: Arc<RwLock<ImageRaw>>,
}

#[derive(Default)]
pub struct AppSinkImageLoader;

impl AssetLoader for AppSinkImageLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            load_context.set_default_asset(LoadedAsset::new(AppSinkImage::new()));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["sinkimage"]
    }
}

impl AppSinkImage {
    pub fn new() -> AppSinkImage {
        let image_raw = Arc::new(RwLock::new([0u8; 176 * 144 * 4]));
        let pipeline = create_pipeline(image_raw.clone()).unwrap();
        pipeline.set_state(gst::State::Playing).unwrap();

        let bus = pipeline
            .bus()
            .expect("Pipeline without bus. Shouldn't happen!");

        AppSinkImage {
            pipeline: pipeline,
            bus: bus,
            image_raw,
        }
    }
}

pub fn create_pipeline(image_raw: Arc<RwLock<ImageRaw>>) -> Result<gst::Pipeline, Error> {
    gst::init()?;

    let pipeline = gst::Pipeline::new(None);
    let src = gst::ElementFactory::make("v4l2src", None).map_err(|_| MissingElement("v4l2src"))?;
    //let src = gst::ElementFactory::make("videotestsrc", None)
    //    .map_err(|_| MissingElement("videotestsrc"))?;
    let dec = gst::ElementFactory::make("jpegdec", None).map_err(|_| MissingElement("jpegdec"))?;
    let sink = gst::ElementFactory::make("appsink", None).map_err(|_| MissingElement("appsink"))?;

    pipeline.add_many(&[&src, &dec, &sink])?;
    src.link(&dec)?;
    dec.link(&sink)?;

    let appsink = sink
        .dynamic_cast::<gst_app::AppSink>()
        .expect("Sink element is expected to be an appsink!");

    // Tell the appsink what format we want. It will then be the audiotestsrc's job to
    // provide the format we request.
    // This can be set after linking the two objects, because format negotiation between
    // both elements will happen during pre-rolling of the pipeline.
    appsink.set_caps(Some(
        &gst::Caps::builder("video/x-raw")
            .field("width", 176)
            .field("height", 144)
            .field("format", "RGB")
            .build(),
    ));

    // Getting data out of the appsink is done by setting callbacks on it.
    // The appsink will then call those handlers, as soon as data is available.
    appsink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            // Add a handler to the "new-sample" signal.
            .new_sample(move |appsink| {
                // Pull the sample in question out of the appsink's buffer.
                let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                let buffer = sample.buffer().ok_or_else(|| {
                    element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to get buffer from appsink")
                    );

                    gst::FlowError::Error
                })?;

                // At this point, buffer is only a reference to an existing memory region somewhere.
                // When we want to access its content, we have to map it while requesting the required
                // mode of access (read, read/write).
                // This type of abstraction is necessary, because the buffer in question might not be
                // on the machine's main memory itself, but rather in the GPU's memory.
                // So mapping the buffer makes the underlying memory region accessible to us.
                // See: https://gstreamer.freedesktop.org/documentation/plugin-development/advanced/allocation.html
                let map = buffer.map_readable().map_err(|_| {
                    element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to map buffer readable")
                    );

                    gst::FlowError::Error
                })?;

                // We know what format the data in the memory region has, since we requested
                // it by setting the appsink's caps. So what we do here is interpret the
                // memory region we mapped as an array of signed 16 bit integers.
                let samples = map.as_slice_of::<u8>().map_err(|_| {
                    element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to interprete buffer as S16 PCM")
                    );

                    gst::FlowError::Error
                })?;

                let mut data = image_raw.write().unwrap();
                for (dest_chunk, src_chunk) in data.chunks_exact_mut(4).zip(samples.chunks_exact(3))
                {
                    dest_chunk[..3].copy_from_slice(src_chunk);
                }

                //println!("ok {} samples", samples.len());

                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );

    Ok(pipeline)
}

fn main_loop(pipeline: gst::Pipeline) -> Result<(), Error> {
    pipeline.set_state(gst::State::Playing)?;

    let bus = pipeline
        .bus()
        .expect("Pipeline without bus. Shouldn't happen!");

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                pipeline.set_state(gst::State::Null)?;
                return Err(ErrorMessage {
                    src: msg
                        .src()
                        .map(|s| String::from(s.path_string()))
                        .unwrap_or_else(|| String::from("None")),
                    error: err.error().to_string(),
                    debug: err.debug(),
                    source: err.error(),
                }
                .into());
            }
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null)?;

    Ok(())
}
