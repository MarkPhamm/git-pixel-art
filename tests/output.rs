use std::sync::OnceLock;

use git_lineage::behavior::Behavior;
use git_lineage::development::derive_pipeline_development;
use git_lineage::history::AccountSnapshot;
use git_lineage::output::{
    GifEncodingError, STATIC_HEIGHT_PIXELS, STATIC_WIDTH_PIXELS, encode_gif, encode_svg,
};
use git_lineage::render::{FRAME_WIDTH_PIXELS, Frame, render_flow_frame, render_still};

#[test]
fn the_gif_rebuilds_every_frame_exactly_at_twice_the_size() {
    let (frames, gif_bytes) = encoded_ingest_excerpt();
    assert_eq!(encode_gif(frames).unwrap(), *gif_bytes);
    let mut gif_options = gif::DecodeOptions::new();
    gif_options.set_color_output(gif::ColorOutput::Indexed);
    let mut gif_decoder = gif_options.read_info(gif_bytes.as_slice()).unwrap();
    assert_eq!(
        (
            u32::from(gif_decoder.width()),
            u32::from(gif_decoder.height())
        ),
        (STATIC_WIDTH_PIXELS, STATIC_HEIGHT_PIXELS)
    );
    // Composite the way a browser does: draw each frame's box over the canvas, show
    // the result, then apply the frame's disposal before the next one.
    let mut canvas = vec![[0; 4]; (STATIC_WIDTH_PIXELS * STATIC_HEIGHT_PIXELS) as usize];
    for frame in frames {
        let gif_frame = gif_decoder.read_next_frame().unwrap().unwrap();
        let palette = gif_frame.palette.as_ref().unwrap();
        let box_pixels = |box_index: usize| {
            let canvas_x = usize::from(gif_frame.left) + box_index % usize::from(gif_frame.width);
            let canvas_y = usize::from(gif_frame.top) + box_index / usize::from(gif_frame.width);
            canvas_y * STATIC_WIDTH_PIXELS as usize + canvas_x
        };
        for (box_index, &palette_index) in gif_frame.buffer.iter().enumerate() {
            if Some(palette_index) != gif_frame.transparent {
                let color_start = usize::from(palette_index) * 3;
                let [red, green, blue] = palette[color_start..color_start + 3] else {
                    unreachable!()
                };
                canvas[box_pixels(box_index)] = [red, green, blue, 255];
            }
        }
        assert_eq!(canvas, scale_to_export(frame));
        assert_eq!(gif_frame.dispose, gif::DisposalMethod::Keep);
    }
    assert!(gif_decoder.read_next_frame().unwrap().is_none());
}

#[test]
fn later_frames_store_only_what_changed() {
    let mut gif_decoder = gif::DecodeOptions::new()
        .read_info(encoded_idle_excerpt().as_slice())
        .unwrap();
    let first_frame = gif_decoder.read_next_frame().unwrap().unwrap();
    assert_eq!(
        (u32::from(first_frame.width), u32::from(first_frame.height)),
        (STATIC_WIDTH_PIXELS, STATIC_HEIGHT_PIXELS)
    );
    while let Some(gif_frame) = gif_decoder.read_next_frame().unwrap() {
        assert!(
            u32::from(gif_frame.width) * u32::from(gif_frame.height)
                < STATIC_WIDTH_PIXELS * STATIC_HEIGHT_PIXELS / 20
        );
    }
}

#[test]
fn the_gif_repeats_forever_at_fifteen_frames_per_second() {
    let mut gif_decoder = gif::DecodeOptions::new()
        .read_info(encoded_ingest_excerpt().1.as_slice())
        .unwrap();
    let mut frame_delays = Vec::new();
    while let Some(gif_frame) = gif_decoder.read_next_frame().unwrap() {
        frame_delays.push(gif_frame.delay);
    }
    assert_eq!(frame_delays, [7, 6, 7, 7, 6, 7]);
    assert_eq!(gif_decoder.repeat(), gif::Repeat::Infinite);
}

#[test]
fn an_empty_loop_is_an_error() {
    assert!(matches!(encode_gif(&[]), Err(GifEncodingError::NoFrames)));
}

#[test]
fn the_svg_draws_the_still_at_the_export_size() {
    let svg_document = encode_svg(&render_still(&read_fixture_development()));
    assert!(svg_document.starts_with(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{STATIC_WIDTH_PIXELS}\" height=\"{STATIC_HEIGHT_PIXELS}\""
    )));
    assert!(svg_document.contains("fill=\"#070a12\""));
    assert!(svg_document.trim_end().ends_with("</svg>"));
}

/// The first six frames of ingest, which pulses the sources and moves packets on every
/// pipe, and their GIF, rendered once for every test.
fn encoded_ingest_excerpt() -> &'static (Vec<Frame>, Vec<u8>) {
    static INGEST_EXCERPT: OnceLock<(Vec<Frame>, Vec<u8>)> = OnceLock::new();
    INGEST_EXCERPT.get_or_init(|| {
        let development = read_fixture_development();
        let frames: Vec<Frame> = (0..6)
            .map(|frame_index| render_flow_frame(&development, Behavior::Ingest, frame_index))
            .collect();
        let gif_bytes = encode_gif(&frames).unwrap();
        (frames, gif_bytes)
    })
}

/// Six idle frames, where only one packet moves.
fn encoded_idle_excerpt() -> &'static Vec<u8> {
    static IDLE_EXCERPT: OnceLock<Vec<u8>> = OnceLock::new();
    IDLE_EXCERPT.get_or_init(|| {
        let development = read_fixture_development();
        let frames: Vec<Frame> = (0..6)
            .map(|frame_index| render_flow_frame(&development, Behavior::Idle, frame_index))
            .collect();
        encode_gif(&frames).unwrap()
    })
}

fn read_fixture_development() -> git_lineage::development::PipelineDevelopment {
    let account_snapshot: AccountSnapshot =
        serde_json::from_str(include_str!("fixtures/account-with-history.json")).unwrap();
    derive_pipeline_development(&account_snapshot).unwrap()
}

/// Repeat each native pixel twice in both dimensions, as the export does.
fn scale_to_export(frame: &Frame) -> Vec<[u8; 4]> {
    let native_pixels = frame.as_rgba_bytes().as_chunks::<4>().0;
    (0..STATIC_WIDTH_PIXELS * STATIC_HEIGHT_PIXELS)
        .map(|export_index| {
            let native_x = export_index % STATIC_WIDTH_PIXELS / 2;
            let native_y = export_index / STATIC_WIDTH_PIXELS / 2;
            native_pixels[(native_y * FRAME_WIDTH_PIXELS + native_x) as usize]
        })
        .collect()
}
