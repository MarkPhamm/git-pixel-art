use std::collections::BTreeMap;

use git_lineage::development::{
    INTERMEDIATE_LAYER_CONTRIBUTIONS, MAX_GROWTH_CONTRIBUTIONS, PipelineDevelopment,
    STAGING_LAYER_CONTRIBUTIONS, derive_pipeline_development,
};
use git_lineage::history::{AccountSnapshot, LanguageColor};
use git_lineage::layout::{DagLayout, Layer, MAX_SOURCE_COUNT, layout_dag};
use git_lineage::render::{FRAME_HEIGHT_PIXELS, FRAME_WIDTH_PIXELS};

#[test]
fn contributions_add_staging_then_intermediate_layers() {
    let layers_at = |total_contributions| {
        layout_dag(&develop(total_contributions, 0x5eed))
            .columns
            .iter()
            .map(|column| column.layer)
            .collect::<Vec<_>>()
    };
    assert_eq!(layers_at(0), [Layer::Source, Layer::Mart]);
    assert_eq!(
        layers_at(STAGING_LAYER_CONTRIBUTIONS - 1),
        [Layer::Source, Layer::Mart]
    );
    assert_eq!(
        layers_at(STAGING_LAYER_CONTRIBUTIONS),
        [Layer::Source, Layer::Staging, Layer::Mart]
    );
    assert_eq!(
        layers_at(INTERMEDIATE_LAYER_CONTRIBUTIONS),
        [
            Layer::Source,
            Layer::Staging,
            Layer::Intermediate,
            Layer::Mart
        ]
    );
    assert_eq!(layers_at(MAX_GROWTH_CONTRIBUTIONS * 10).len(), 4);
}

#[test]
fn models_only_widen_as_contributions_grow() {
    let mut previous_node_count = 0;
    for total_contributions in [
        0, 100, 250, 1_000, 2_500, 10_000, 50_000, 100_000, 1_000_000,
    ] {
        let node_count = layout_dag(&develop(total_contributions, 0x5eed))
            .nodes
            .len();
        assert!(
            node_count >= previous_node_count,
            "{total_contributions} contributions"
        );
        previous_node_count = node_count;
    }
}

#[test]
fn the_leading_languages_become_sources_as_tall_as_their_shares() {
    let layout = layout_dag(&develop(10_000, 0x5eed));
    let source_names: Vec<_> = layout
        .sources
        .iter()
        .map(|source| source.language_name.as_deref().unwrap())
        .collect();
    assert_eq!(source_names.len(), MAX_SOURCE_COUNT);
    assert_eq!(
        source_names,
        ["Python", "Jupyter Notebook", "HTML", "HCL", "TypeScript"]
    );
    let source_heights: Vec<i32> = layout.nodes[..MAX_SOURCE_COUNT]
        .iter()
        .map(|node| node.rect[3])
        .collect();
    assert!(source_heights.windows(2).all(|pair| pair[0] >= pair[1]));
    assert!(source_heights[0] > source_heights[MAX_SOURCE_COUNT - 1]);
    assert_eq!(
        layout.sources[2].color,
        Some(LanguageColor([0xe3, 0x4c, 0x26]))
    );
}

#[test]
fn an_account_without_languages_gets_one_uncolored_source() {
    let mut development = develop(10_000, 0x5eed);
    development.language_shares.clear();
    development.language_colors.clear();
    let layout = layout_dag(&development);
    assert_eq!(layout.sources.len(), 1);
    assert_eq!(layout.sources[0].language_name, None);
    assert_eq!(layout.sources[0].color, None);
    assert_well_formed(&layout);
}

#[test]
fn every_graph_is_connected_and_fits_the_canvas() {
    for total_contributions in [0, 300, 3_000, 100_000] {
        for identity_seed in 0..40 {
            assert_well_formed(&layout_dag(&develop(total_contributions, identity_seed)));
        }
    }
}

#[test]
fn each_staging_model_reads_its_own_source() {
    let layout = layout_dag(&develop(STAGING_LAYER_CONTRIBUTIONS, 7));
    let staging_nodes: Vec<usize> = (0..layout.nodes.len())
        .filter(|&node_index| layout.nodes[node_index].layer == Layer::Staging)
        .collect();
    assert_eq!(staging_nodes.len(), layout.sources.len());
    for (source_index, &staging_node) in staging_nodes.iter().enumerate() {
        let parents: Vec<usize> = layout
            .edges
            .iter()
            .filter(|edge| edge.to == staging_node)
            .map(|edge| edge.from)
            .collect();
        assert_eq!(parents, [source_index]);
        assert_eq!(layout.nodes[staging_node].lineage, source_index);
    }
}

#[test]
fn the_seed_decides_the_wiring_and_growth_keeps_the_sources_in_place() {
    let development = develop(50_000, 0x5eed);
    assert_eq!(layout_dag(&development), layout_dag(&development));
    let wirings: Vec<Vec<(usize, usize)>> = (0..20)
        .map(|identity_seed| {
            layout_dag(&develop(50_000, identity_seed))
                .edges
                .iter()
                .map(|edge| (edge.from, edge.to))
                .collect()
        })
        .collect();
    assert!(wirings.iter().any(|wiring| *wiring != wirings[0]));
    let young_sources = layout_dag(&develop(10, 0x5eed)).nodes[..MAX_SOURCE_COUNT].to_vec();
    let grown_sources = layout_dag(&develop(100_000, 0x5eed)).nodes[..MAX_SOURCE_COUNT].to_vec();
    assert_eq!(young_sources, grown_sources);
}

/// Every node and pipe fits the canvas; pipes run in unit steps from a parent's right
/// side to a child's left side in the next column; every node before the marts feeds a
/// child, and every node after the sources has a parent.
fn assert_well_formed(layout: &DagLayout) {
    let fits = |x: i32, y: i32| {
        (0..FRAME_WIDTH_PIXELS as i32).contains(&x) && (0..FRAME_HEIGHT_PIXELS as i32).contains(&y)
    };
    for node in &layout.nodes {
        let [x, y, width, height] = node.rect;
        assert!(
            fits(x, y) && fits(x + width - 1, y + height - 1),
            "{node:?}"
        );
    }
    for edge in &layout.edges {
        let parent = &layout.nodes[edge.from];
        let child = &layout.nodes[edge.to];
        assert_eq!(
            layout.columns[edge.gap].layer, parent.layer,
            "a pipe leaves the column of its gap"
        );
        assert_eq!(layout.columns[edge.gap + 1].layer, child.layer);
        let first_pixel = edge.pixels[0];
        let last_pixel = *edge.pixels.last().unwrap();
        assert_eq!(first_pixel[0], parent.rect[0] + parent.rect[2]);
        assert_eq!(last_pixel[0], child.rect[0] - 1);
        for pair in edge.pixels.windows(2) {
            assert_eq!(
                (pair[0][0] - pair[1][0]).abs() + (pair[0][1] - pair[1][1]).abs(),
                1
            );
        }
        assert!(edge.pixels.iter().all(|&[x, y]| fits(x, y)));
    }
    for (node_index, node) in layout.nodes.iter().enumerate() {
        if node.layer != Layer::Mart {
            assert!(layout.edges.iter().any(|edge| edge.from == node_index));
        }
        if node.layer != Layer::Source {
            assert!(layout.edges.iter().any(|edge| edge.to == node_index));
        }
    }
}

fn develop(total_contributions: u64, identity_seed: u64) -> PipelineDevelopment {
    let account_snapshot: AccountSnapshot =
        serde_json::from_str(include_str!("fixtures/account-with-history.json")).unwrap();
    let mut development = derive_pipeline_development(&account_snapshot).unwrap();
    let language_mix = [
        ("Python", [0x35, 0x72, 0xa5], 0.4),
        ("Jupyter Notebook", [0xda, 0x5b, 0x0b], 0.2),
        ("HTML", [0xe3, 0x4c, 0x26], 0.14),
        ("HCL", [0x84, 0x4f, 0xba], 0.1),
        ("TypeScript", [0x31, 0x78, 0xc6], 0.1),
        ("CSS", [0x66, 0x33, 0x99], 0.06),
    ];
    development.language_shares = language_mix
        .iter()
        .map(|&(language_name, _, share)| (language_name.to_owned(), share))
        .collect();
    development.language_colors = language_mix
        .iter()
        .map(|&(language_name, channels, _)| {
            (language_name.to_owned(), Some(LanguageColor(channels)))
        })
        .collect::<BTreeMap<_, _>>();
    development.total_contributions = total_contributions;
    development.identity_seed = identity_seed;
    development
}
