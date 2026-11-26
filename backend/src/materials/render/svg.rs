//! Draws a [`Diagram`] as a self-contained SVG: nodes on a circle (grouped
//! nodes sit next to each other, one colour per group), edges as straight
//! lines with arrowheads. Deterministic and dependency-free, so HTML, EPUB and
//! PDF (via the HTML renderer) all show the same picture.

use std::f64::consts::PI;

use crate::materials::blocks::Diagram;

const SIZE: f64 = 640.0;
const MAX_NODES: usize = 24;
const PALETTE: [&str; 6] = [
    "#2f6fdd", "#c4552b", "#2e8b57", "#8a4fbf", "#b8860b", "#2a8c9c",
];

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn short(label: &str) -> String {
    let chars: Vec<char> = label.chars().collect();
    if chars.len() <= 22 {
        label.to_owned()
    } else {
        format!("{}…", chars[..21].iter().collect::<String>())
    }
}

/// SVG markup, or `None` when there is nothing to draw.
pub fn render_diagram(diagram: &Diagram, alt: &str) -> Option<String> {
    if diagram.nodes.is_empty() {
        return None;
    }
    // Group-ordered so each group occupies one arc.
    let mut order: Vec<usize> = (0..diagram.nodes.len()).collect();
    order.sort_by_key(|&i| diagram.nodes[i].group.clone().unwrap_or_default());
    order.truncate(MAX_NODES);

    let mut groups: Vec<String> = Vec::new();
    for &i in &order {
        let g = diagram.nodes[i].group.clone().unwrap_or_default();
        if !groups.contains(&g) {
            groups.push(g);
        }
    }

    let n = order.len();
    let (cx, cy) = (SIZE / 2.0, SIZE / 2.0);
    let radius = if n == 1 { 0.0 } else { SIZE / 2.0 - 90.0 };
    let pos = |slot: usize| -> (f64, f64) {
        let a = 2.0 * PI * slot as f64 / n as f64 - PI / 2.0;
        (cx + radius * a.cos(), cy + radius * a.sin())
    };
    let slot_of = |id: &str| order.iter().position(|&i| diagram.nodes[i].id == id);

    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"-120 0 {} {SIZE}\" role=\"img\" aria-label=\"{}\" style=\"max-width:100%;height:auto\">",
        SIZE + 240.0,
        esc(alt)
    );
    out.push_str(
        "<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto-start-reverse\"><path d=\"M0,0 L10,5 L0,10 z\" fill=\"#8a909c\"/></marker></defs>",
    );
    for edge in &diagram.edges {
        let (Some(a), Some(b)) = (slot_of(&edge.from), slot_of(&edge.to)) else {
            continue;
        };
        if a == b {
            continue;
        }
        let ((x1, y1), (x2, y2)) = (pos(a), pos(b));
        // Stop short of the node circles so arrowheads stay visible.
        let (dx, dy) = (x2 - x1, y2 - y1);
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let (ux, uy) = (dx / len, dy / len);
        out.push_str(&format!(
            "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#8a909c\" stroke-width=\"1.3\" marker-end=\"url(#arrow)\"/>",
            x1 + ux * 16.0, y1 + uy * 16.0, x2 - ux * 18.0, y2 - uy * 18.0
        ));
        if let Some(label) = edge.label.as_deref().filter(|l| !l.is_empty()) {
            out.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"9\" fill=\"#5b6270\" text-anchor=\"middle\">{}</text>",
                (x1 + x2) / 2.0,
                (y1 + y2) / 2.0 - 2.0,
                esc(&short(label))
            ));
        }
    }
    for (slot, &i) in order.iter().enumerate() {
        let node = &diagram.nodes[i];
        let g = node.group.clone().unwrap_or_default();
        let color = PALETTE[groups.iter().position(|x| *x == g).unwrap_or(0) % PALETTE.len()];
        let (x, y) = pos(slot);
        // Labels on the outer side of the circle, anchored away from it.
        let anchor = if x < cx - 10.0 {
            "end"
        } else if x > cx + 10.0 {
            "start"
        } else {
            "middle"
        };
        let dx = if anchor == "end" {
            -14.0
        } else if anchor == "start" {
            14.0
        } else {
            0.0
        };
        let dy = if anchor == "middle" {
            if y < cy {
                -14.0
            } else {
                22.0
            }
        } else {
            4.0
        };
        out.push_str(&format!(
            "<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"10\" fill=\"{color}\"/><text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\" fill=\"#1c1f26\" text-anchor=\"{anchor}\">{}</text>",
            x + dx, y + dy, esc(&short(&node.label))
        ));
    }
    out.push_str("</svg>");
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::blocks::{DiagramEdge, DiagramNode};

    fn node(id: &str, label: &str, group: &str) -> DiagramNode {
        DiagramNode {
            id: id.into(),
            label: label.into(),
            group: Some(group.into()),
        }
    }

    #[test]
    fn draws_nodes_edges_and_escapes() {
        let d = Diagram {
            nodes: vec![
                node("a", "Heat <flux>", "Physics"),
                node("b", "Fourier", "Laws"),
            ],
            edges: vec![
                DiagramEdge {
                    from: "a".into(),
                    to: "b".into(),
                    label: Some("obeys".into()),
                },
                DiagramEdge {
                    from: "a".into(),
                    to: "missing".into(),
                    label: None,
                },
            ],
        };
        let svg = render_diagram(&d, "A map").unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Heat &lt;flux&gt;"));
        assert_eq!(svg.matches("<line").count(), 1, "dangling edge is skipped");
        assert_eq!(svg.matches("<circle").count(), 2);
        assert!(svg.contains("obeys"));
    }

    #[test]
    fn empty_and_single_node() {
        assert!(render_diagram(
            &Diagram {
                nodes: vec![],
                edges: vec![]
            },
            "x"
        )
        .is_none());
        let one = Diagram {
            nodes: vec![node("a", "Only", "G")],
            edges: vec![],
        };
        assert!(render_diagram(&one, "x").unwrap().contains("Only"));
    }
}
