use pulldown_cmark_wikilink::{ParserOffsetIter, LinkType, Event};
pub use pulldown_cmark_wikilink::Options;
use web_sys::MouseEvent;

use core::ops::Range;
use std::collections::HashMap;

mod render;
use render::Renderer;

mod utils;

mod component;


pub struct ElementAttributes<'a, F: Context<'a>> {
    pub classes: Vec<String>,
    pub style: Option<String>,
    pub inner_html: Option<String>,
    pub on_click: Option<F::Handler<'a, MouseEvent>>
}

impl<'a, F: Context<'a>> Default for ElementAttributes<'a,  F> {
    fn default() -> Self {
        Self {
            style: None,
            classes: vec![],
            inner_html: None,
            on_click: None
        }
    }
}

pub enum HtmlElement {
    Div,
    Span,
    Paragraph,
    BlockQuote,
    Ul,
    Ol(i32),
    Li,
    Heading(u8),
    Table,
    Thead,
    Trow,
    Tcell,
    Italics,
    Bold,
    StrikeThrough,
    Pre,
    Code
}

pub trait Context<'a>: 'a + Clone {
    type View;
    type HtmlCallback<T: 'a>: Clone + 'a;
    type Handler<'b, T: 'b>: Clone + 'b;
    type Setter<T>: Clone;
    fn props(&'a self) -> MarkdownProps<'a, Self>;
    fn set<T>(&self, setter: &Self::Setter<T>, value: T);
    fn send_debug_info(&self, info: Vec<String>);
    fn el_with_attributes(&self, e: HtmlElement, inside: Self::View, attributes: ElementAttributes<'a, Self>) -> Self::View;
    fn el(&self, e: HtmlElement, inside: Self::View) -> Self::View {
        self.el_with_attributes(e, inside, Default::default())
    }
    fn el_hr(&self, attributes: ElementAttributes<'a, Self>) -> Self::View;
    fn el_br(&self)-> Self::View;
    fn el_fragment(&self, children: Vec<Self::View>) -> Self::View;
    fn el_a(&self, children: Self::View, href: &str) -> Self::View;
    fn el_img(&self, src: &str, alt: &str) -> Self::View;
    fn el_empty(&self) -> Self::View {
        self.el_fragment(vec![])
    }
    fn el_text(&self, text: &str) -> Self::View;
    fn mount_dynamic_link(&self, rel: &str, href: &str, integrity: &str, crossorigin: &str);
    fn el_input_checkbox(&self, checked: bool, attributes: ElementAttributes<'a, Self>) -> Self::View;
    fn call_handler<'b, T>(&self, callback: &Self::Handler<'b, T>, input: T);
    fn call_html_callback<T>(&self, callback: &Self::HtmlCallback<T>, input: T) -> Self::View;
    fn make_handler<'b, T, F: Fn(T)>(&self, f: F) -> Self::Handler<'b, T>;

    fn make_md_callback<'b>(&'a self, position: Range<usize>) 
        -> Self::Handler<'b, MouseEvent>
    {
        let callback = self.props().on_click.cloned();
        let f = move |x| {
            let click_event = MarkdownMouseEvent {
                mouse_event: x,
                position: position.clone()
            };
            match &callback {
                Some(cb) => self.call_handler(&cb, click_event),
                _ => ()
            }
        };
        self.make_handler(f)
    }

    fn render_tasklist_marker(&'a self, m: bool, position: Range<usize>) 
        -> Self::View {
        let callback = self.props().on_click.cloned();
        let callback = move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();
            let click_event = MarkdownMouseEvent {
                mouse_event: e,
                position: position.clone()
            };
            if let Some(cb) = callback.clone() {
                self.call_handler(&cb, click_event)
            }
        };

        let attributes = ElementAttributes {
            on_click: Some(self.make_handler(callback)),
            ..Default::default()
        };
        self.el_input_checkbox(m, attributes)
    }

    fn render_rule(&'a self, range: Range<usize>) -> Self::View {
        let attributes = ElementAttributes{
            on_click: Some(self.make_md_callback(range)),
            ..Default::default()
        };
        self.el_hr(attributes)
    }


    fn render_code(&'a self, s: &str, range: Range<usize>) -> Self::View {
        let callback = self.make_md_callback(range.clone());
        let attributes = ElementAttributes{
            on_click: Some(callback),
            ..Default::default()
        };
        self.el_with_attributes(HtmlElement::Code, self.el_text(s), attributes)
    }


    fn render_text(&'a self, s: &str, range: Range<usize>) -> Self::View{
        let callback = self.make_md_callback(range);
        let attributes = ElementAttributes{
            on_click: Some(callback),
            ..Default::default()
        };
        self.el_with_attributes(HtmlElement::Span, self.el_text(s), attributes)
    }


    fn render_link(&'a self, link: LinkDescription<'a, Self>) 
        -> Self::View 
    {
        match (&self.props().render_links, link.image) {
            (Some(f), _) => self.call_html_callback(&f, link),
            (None, false) => self.el_a(link.content, &link.url),
            (None, true) => self.el_img(&link.url, &link.title),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MarkdownMouseEvent {
    /// the original mouse event triggered when a text element was clicked on
    pub mouse_event: MouseEvent,

    /// the corresponding range in the markdown source, as a slice of [`u8`][u8]
    pub position: Range<usize>,

    // TODO: add a clonable tag for the type of the element
    // pub tag: pulldown_cmark::Tag<'a>,
}


/// the description of a link, used to render it with a custom callback.
/// See [pulldown_cmark::Tag::Link] for documentation
pub struct LinkDescription<'a, F: Context<'a>> {
    /// the url of the link
    pub url: String,

    /// the html view of the element under the link
    pub content: F::View,

    /// the title of the link. 
    /// If you don't know what it is, don't worry: it is ofter empty
    pub title: String,

    /// the type of link
    pub link_type: LinkType,

    /// wether the link is an image
    pub image: bool,
}


#[derive(PartialEq)]
pub struct MdComponentProps<'a, F: Context<'a>> {
    pub attributes: Vec<(String, String)>,
    pub children: F::View
}


#[derive(Clone)]
pub struct MarkdownProps<'a, F: Context<'a>>
{
    pub on_click: Option<&'a F::Handler<'a, MarkdownMouseEvent>>,

    pub render_links: Option<&'a F::HtmlCallback<LinkDescription<'a, F>>>,

    pub theme: Option<&'a str>,

    pub wikilinks: bool,

    pub hard_line_breaks: bool,

    pub parse_options: Option<&'a pulldown_cmark_wikilink::Options>,

    pub components: &'a HashMap<String, F::HtmlCallback<MdComponentProps<'a, F>>>,

    pub frontmatter: Option<&'a F::Setter<String>>
}

impl<'a, F: Context<'a>> Copy for MarkdownProps<'a, F> {}

pub fn render_markdown<'a, F: Context<'a>>(
    cx: &'a F, 
    source: &'a str, 
    ) -> F::View {

    let parse_options_default = Options::all();
    let options = cx.props().parse_options.unwrap_or(&parse_options_default);
    let mut stream: Vec<_>
        = ParserOffsetIter::new_ext(source, *options, cx.props().wikilinks).collect();

    if cx.props().hard_line_breaks {
        for (r, _) in &mut stream {
            if *r == Event::SoftBreak {
                *r = Event::HardBreak
            }
        }
    }

    let elements = Renderer::new(cx, &mut stream.into_iter())
        .collect::<Vec<_>>();


    cx.mount_dynamic_link(
        "stylesheet",
        "https://cdn.jsdelivr.net/npm/katex@0.16.7/dist/katex.min.css",
        "sha384-3UiQGuEI4TTMaFmGIZumfRPtfKQ3trwQE2JgosJxCnGmQpL/lJdjpcHkaaFwHlcI",
        "anonymous"
    );

    cx.el_fragment(elements)
}
