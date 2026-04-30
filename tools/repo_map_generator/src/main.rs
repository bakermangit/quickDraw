use anyhow::{Context, Result};
use quote::ToTokens;
use std::fs;
use std::path::Path;
use syn::visit::Visit;
use syn::{Attribute, ItemEnum, ItemFn, ItemStruct, ItemTrait, ItemType, Visibility};
use walkdir::{DirEntry, WalkDir};

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

fn is_test_dir(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s == "tests")
        .unwrap_or(false)
}

fn has_test_attr(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if attr.meta.path().is_ident("test") {
            return true;
        }
        if let syn::Meta::List(meta) = &attr.meta {
            if meta.path.is_ident("cfg") {
                let tokens = meta.tokens.to_string();
                if tokens.contains("test") {
                    return true;
                }
            }
        }
    }
    false
}

fn extract_docstrings(attrs: &[Attribute]) -> String {
    let mut docs = String::new();
    for attr in attrs {
        if attr.meta.path().is_ident("doc") {
            if let syn::Meta::NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        let doc = lit_str.value();
                        docs.push_str("///");
                        docs.push_str(&doc);
                        docs.push('\n');
                    }
                }
            }
        }
    }
    docs
}

#[derive(Default)]
struct PublicItemVisitor {
    items: Vec<String>,
}

impl<'ast> Visit<'ast> for PublicItemVisitor {
    fn visit_item_struct(&mut self, node: &'ast ItemStruct) {
        if matches!(node.vis, Visibility::Public(_)) && !has_test_attr(&node.attrs) {
            let mut out = extract_docstrings(&node.attrs);
            let mut clone_node = node.clone();
            clone_node.attrs.clear();
            out.push_str(&clone_node.into_token_stream().to_string());
            self.items.push(out);
        }
        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast ItemEnum) {
        if matches!(node.vis, Visibility::Public(_)) && !has_test_attr(&node.attrs) {
            let mut out = extract_docstrings(&node.attrs);
            let mut clone_node = node.clone();
            clone_node.attrs.clear();
            out.push_str(&clone_node.into_token_stream().to_string());
            self.items.push(out);
        }
        syn::visit::visit_item_enum(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast ItemTrait) {
        if matches!(node.vis, Visibility::Public(_)) && !has_test_attr(&node.attrs) {
            let mut out = extract_docstrings(&node.attrs);
            let mut clone_node = node.clone();
            clone_node.attrs.clear();
            out.push_str(&clone_node.into_token_stream().to_string());
            self.items.push(out);
        }
        syn::visit::visit_item_trait(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if matches!(node.vis, Visibility::Public(_)) && !has_test_attr(&node.attrs) {
            let mut out = extract_docstrings(&node.attrs);
            let vis = node.vis.to_token_stream().to_string();
            let sig = node.sig.to_token_stream().to_string();
            out.push_str(&format!("{} {};", vis, sig));
            self.items.push(out);
        }
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_item_type(&mut self, node: &'ast ItemType) {
        if matches!(node.vis, Visibility::Public(_)) && !has_test_attr(&node.attrs) {
            let mut out = extract_docstrings(&node.attrs);
            let mut clone_node = node.clone();
            clone_node.attrs.clear();
            out.push_str(&clone_node.into_token_stream().to_string());
            self.items.push(out);
        }
        syn::visit::visit_item_type(self, node);
    }
}

fn process_file(path: &Path) -> Result<Option<String>> {
    let content = fs::read_to_string(path)?;
    let syntax_tree = syn::parse_file(&content)?;
    
    let mut visitor = PublicItemVisitor::default();
    visitor.visit_file(&syntax_tree);
    
    if visitor.items.is_empty() {
        Ok(None)
    } else {
        Ok(Some(visitor.items.join("\n\n")))
    }
}

fn main() -> Result<()> {
    let src_dir = Path::new("src");
    if !src_dir.exists() {
        anyhow::bail!("src/ directory not found. Please run this command from the project root.");
    }

    let mut output = String::new();
    output.push_str("<repo_map>\n");

    let walker = WalkDir::new(src_dir)
        .into_iter()
        .filter_entry(|e| !is_hidden(e) && !is_test_dir(e));

    let mut files = Vec::new();
    for entry in walker {
        let entry = entry?;
        if entry.file_type().is_file() && entry.path().extension().map_or(false, |e| e == "rs") {
            files.push(entry.path().to_path_buf());
        }
    }
    
    files.sort();

    for file in files {
        if let Some(items) = process_file(&file).with_context(|| format!("Failed to process {}", file.display()))? {
            let path_str = file.to_string_lossy().replace('\\', "/");
            output.push_str(&format!("### {}\n```rust\n{}\n```\n\n", path_str, items));
        }
    }

    output.push_str("</repo_map>\n");

    let out_path = Path::new("docs/REPO_MAP.md");
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out_path, output)?;

    println!("Successfully generated {}", out_path.display());

    Ok(())
}
