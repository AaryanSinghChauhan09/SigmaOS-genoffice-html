#[derive(Debug, Clone)]
pub struct HtmlElement {
    pub tag: String,
    pub id: Option<String>,
    pub class: Option<String>,
    pub inner_text: String,
}

impl HtmlElement {
    pub fn new(tag: impl Into<String>, inner_text: impl Into<String>) -> Self {
        Self { tag: tag.into(), id: None, class: None, inner_text: inner_text.into() }
    }
    pub fn with_id(mut self, id: impl Into<String>) -> Self { self.id = Some(id.into()); self }
    pub fn render(&self) -> String {
        let id_attr = self.id.as_ref().map(|v| format!(" id="{}"", v)).unwrap_or_default();
        format!("<{tag}{id}>{text}</{tag}>", tag=self.tag, id=id_attr, text=self.inner_text)
    }
}

#[derive(Debug, Default)]
pub struct HtmlDocument {
    pub title: String,
    elements: Vec<HtmlElement>,
}

impl HtmlDocument {
    pub fn new(title: impl Into<String>) -> Self { Self { title: title.into(), elements: Vec::new() } }
    pub fn append(&mut self, el: HtmlElement) { self.elements.push(el); }
    pub fn element_count(&self) -> usize { self.elements.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn element_render() {
        let el = HtmlElement::new("h1", "Hello SigmaOS").with_id("title");
        assert!(el.render().contains("id="title""));
    }
}
