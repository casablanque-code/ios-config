// Pass 1: build an indent-aware tree from raw config text.
// No IOS semantics at this stage — purely whitespace-based hierarchy.

#[derive(Debug, Clone)]
pub struct RawNode {
    pub line_num: usize,
    pub indent: usize,
    pub text: String,
    pub children: Vec<RawNode>,
}

impl RawNode {
    pub fn new(line_num: usize, indent: usize, text: &str) -> Self {
        RawNode {
            line_num,
            indent,
            text: text.to_string(),
            children: vec![],
        }
    }

    /// First whitespace-separated token — the command keyword.
    pub fn keyword(&self) -> &str {
        self.text.split_whitespace().next().unwrap_or("")
    }

    /// All tokens after the first.
    pub fn args(&self) -> Vec<&str> {
        self.text.split_whitespace().skip(1).collect()
    }

    /// Full trimmed command text.
    pub fn full(&self) -> &str {
        &self.text
    }
}

fn leading_spaces(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Root of the parsed config tree — a list of top-level nodes.
#[derive(Debug)]
pub struct RawTree {
    pub nodes: Vec<RawNode>,
}

pub fn parse_raw_tree(input: &str) -> RawTree {
    let mut flat: Vec<RawNode> = vec![];

    for (line_num, line) in input.lines().enumerate() {
        let trimmed_end = line.trim_end();
        if trimmed_end.is_empty() {
            continue;
        }
        // Skip comment lines and section separators.
        let stripped = trimmed_end.trim_start_matches('!');
        if stripped.trim().is_empty() || trimmed_end.trim_start().starts_with('!') {
            continue;
        }

        let indent = leading_spaces(trimmed_end);
        let text = trimmed_end.trim();
        flat.push(RawNode::new(line_num + 1, indent, text));
    }

    build_tree(flat)
}

fn build_tree(flat: Vec<RawNode>) -> RawTree {
    if flat.is_empty() {
        return RawTree { nodes: vec![] };
    }
    let nodes = assemble(&flat, 0, 0).0;
    RawTree { nodes }
}

/// Recursively groups `flat[idx..]` into a tree.
/// Nodes with `indent > min_indent` become children of the preceding sibling.
fn assemble(flat: &[RawNode], idx: usize, min_indent: usize) -> (Vec<RawNode>, usize) {
    let mut result: Vec<RawNode> = vec![];
    let mut i = idx;

    while i < flat.len() {
        let node = &flat[i];

        if node.indent < min_indent {
            break;
        }

        if node.indent > min_indent {
            if let Some(parent) = result.last_mut() {
                let (children, consumed) = assemble(flat, i, node.indent);
                parent.children.extend(children);
                i += consumed;
            } else {
                // Orphaned indented line (non-standard config) — treat as top-level.
                let mut orphan = flat[i].clone();
                orphan.children = vec![];
                result.push(orphan);
                i += 1;
            }
        } else {
            let mut new_node = flat[i].clone();
            new_node.children = vec![];
            result.push(new_node);
            i += 1;
        }
    }

    (result, i - idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_interface_block() {
        let input = r#"
hostname ROUTER-01
!
interface GigabitEthernet0/0
 description WAN
 ip address 10.0.0.1 255.255.255.0
 no shutdown
!
interface GigabitEthernet0/1
 description LAN
 ip address 192.168.1.1 255.255.255.0
 shutdown
"#;
        let tree = parse_raw_tree(input);
        assert_eq!(tree.nodes.len(), 3); // hostname + 2 interfaces
        assert_eq!(tree.nodes[1].keyword(), "interface");
        assert_eq!(tree.nodes[1].children.len(), 3);
    }
}
