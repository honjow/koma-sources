// Exact data files from adopted com.luhuiguo:chinese-utils:1.0 (Apache2).
// Independent longest-prefix implementation of Converter.SIMPLIFIED; no
// replacement subset and no runtime library/process/network dependency.
use super::*;
use alloc::collections::BTreeMap;
struct Node {
    children: BTreeMap<char, usize>,
    value: Option<String>,
}
pub struct Simplifier {
    chars: Vec<char>,
    nodes: Vec<Node>,
}
impl Simplifier {
    pub fn new() -> R<Self> {
        let chars = include_str!("../data/chinese-utils-1.0/simp.txt")
            .lines()
            .flat_map(str::chars)
            .collect::<Vec<_>>();
        if chars.len() != 0x9fa5 - 0x4e00 + 1 {
            return Err("internal_error".into());
        }
        let mut result = Self {
            chars,
            nodes: alloc::vec![Node {
                children: BTreeMap::new(),
                value: None
            }],
        };
        for line in include_str!("../data/chinese-utils-1.0/simplified.txt").lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split('=');
            let key = parts.next().unwrap_or("");
            let Some(value) = parts.next() else { continue };
            if key.is_empty() {
                continue;
            }
            let mut index = 0;
            for c in key.chars() {
                let next = if let Some(n) = result.nodes[index].children.get(&c) {
                    *n
                } else {
                    let n = result.nodes.len();
                    result.nodes.push(Node {
                        children: BTreeMap::new(),
                        value: None,
                    });
                    result.nodes[index].children.insert(c, n);
                    n
                };
                index = next
            }
            result.nodes[index].value = Some(value.to_string())
        }
        Ok(result)
    }
    pub fn convert(&self, s: &str) -> String {
        let chars = s.chars().collect::<Vec<_>>();
        let mut pos = 0;
        let mut out = String::new();
        while pos < chars.len() {
            let mut node = 0;
            let mut best = None;
            for (n, c) in chars[pos..].iter().enumerate() {
                let Some(next) = self.nodes[node].children.get(c) else {
                    break;
                };
                node = *next;
                if let Some(value) = self.nodes[node].value.as_deref() {
                    best = Some((pos + n + 1, value))
                }
            }
            if let Some((next, value)) = best {
                out.push_str(value);
                pos = next
            } else {
                let c = chars[pos];
                out.push(if ('\u{4e00}'..='\u{9fa5}').contains(&c) {
                    self.chars[c as usize - 0x4e00]
                } else {
                    c
                });
                pos += 1
            }
        }
        out
    }
}
