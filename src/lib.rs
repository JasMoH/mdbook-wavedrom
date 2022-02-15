use mdbook::book::{Book, BookItem, Chapter};
use mdbook::errors::Result;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use pulldown_cmark::{CodeBlockKind::*, Event, Options, Parser, Tag};

pub struct Wavedrom;

impl Preprocessor for Wavedrom {
    fn name(&self) -> &str {
        "wavedrom"
    }

    fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
        let mut res = None;
        book.for_each_mut(|item: &mut BookItem| {
            if let Some(Err(_)) = res {
                return;
            }

            if let BookItem::Chapter(ref mut chapter) = *item {
                res = Some(Wavedrom::add_wavedrom(chapter).map(|md| {
                    chapter.content = md;
                }));
            }
        });

        res.unwrap_or(Ok(())).map(|_| book)
    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        renderer == "html"
    }
}

fn escape_html(s: &str) -> String {
    let mut output = String::new();
    for c in s.chars() {
        match c {
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '&' => output.push_str("&amp;"),
            _ => output.push(c),
        }
    }
    output
}

fn add_wavedrom(content: &str) -> Result<String> {
    let mut wavedrom_content = String::new();
    let mut in_wavedrom_block = false;

    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);

    let mut wavedrom_start = 0..0;

    let mut wavedrom_blocks = vec![];

    let events = Parser::new_ext(content, opts);
    for (e, span) in events.into_offset_iter() {
        if let Event::Start(Tag::CodeBlock(Fenced(code))) = e.clone() {
            log::debug!("e={:?}, span={:?}", e, span);
            if &*code == "wavedrom" {
                wavedrom_start = span;
                in_wavedrom_block = true;
                wavedrom_content.clear();
            }
            continue;
        }

        if !in_wavedrom_block {
            continue;
        }

        if let Event::End(Tag::CodeBlock(Fenced(code))) = e {
            assert_eq!(
                "wavedrom", &*code,
                "After an opening wavedrom code block we expect it to close again"
            );
            in_wavedrom_block = false;
            let pre = "```wavedrom\n";
            let post = "```";

            let wavedrom_content = &content[wavedrom_start.start + pre.len()..span.end - post.len()];
            let wavedrom_content = escape_html(wavedrom_content);
            let wavedrom_code = format!("<body onload=\"WaveDrom.ProcessAll()\">\n\n<script type=\"WaveDrom\">{}</script>\n\n", wavedrom_content);
            wavedrom_blocks.push((wavedrom_start.start..span.end, wavedrom_code.clone()));
        }
    }

    let mut content = content.to_string();
    for (span, block) in wavedrom_blocks.iter().rev() {
        let pre_content = &content[0..span.start];
        let post_content = &content[span.end..];
        content = format!("{}\n{}{}", pre_content, block, post_content);
    }
    Ok(content)
}

impl Wavedrom {
    fn add_wavedrom(chapter: &mut Chapter) -> Result<String> {
        add_wavedrom(&chapter.content)
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;

    use super::add_wavedrom;

    #[test]
    fn adds_wavedrom() {
        let content = r#"# Chapter

```wavedrom
{signal: [
  {name: 'clk', wave: 'p.....|...'}
]}
```

Text
"#;

        let expected = r#"# Chapter


<body onload="WaveDrom.ProcessAll()">

<script type="WaveDrom">{signal: [
  {name: 'clk', wave: 'p.....|...'}
]}
</script>



Text
"#;

        assert_eq!(expected, add_wavedrom(content).unwrap());
    }

    #[test]
    fn leaves_tables_untouched() {
        // Regression test.
        // Previously we forgot to enable the same markdwon extensions as mdbook itself.

        let content = r#"# Heading

| Head 1 | Head 2 |
|--------|--------|
| Row 1  | Row 2  |
"#;

        let expected = r#"# Heading

| Head 1 | Head 2 |
|--------|--------|
| Row 1  | Row 2  |
"#;

        assert_eq!(expected, add_wavedrom(content).unwrap());
    }

    #[test]
    fn leaves_html_untouched() {
        // Regression test.
        // Don't remove important newlines for syntax nested inside HTML

        let content = r#"# Heading

<del>

*foo*

</del>
"#;

        let expected = r#"# Heading

<del>

*foo*

</del>
"#;

        assert_eq!(expected, add_wavedrom(content).unwrap());
    }

    #[test]
    fn html_in_list() {
        // Regression test.
        // Don't remove important newlines for syntax nested inside HTML

        let content = r#"# Heading

1. paragraph 1
   ```
   code 1
   ```
2. paragraph 2
"#;

        let expected = r#"# Heading

1. paragraph 1
   ```
   code 1
   ```
2. paragraph 2
"#;

        assert_eq!(expected, add_wavedrom(content).unwrap());
    }

    #[test]
    fn escape_in_wavedrom_block() {
        env_logger::init();
        let content = r#"
```wavedrom
classDiagram
    class PingUploader {
        <<interface>>
        +Upload() UploadResult
    }
```

hello
"#;

        let expected = r#"

<body onload="WaveDrom.ProcessAll()">

<script type="WaveDrom">classDiagram
    class PingUploader {
        &lt;&lt;interface&gt;&gt;
        +Upload() UploadResult
    }
</script>



hello
"#;

        assert_eq!(expected, add_wavedrom(content).unwrap());
    }

//    #[test]
//    fn adds_body_onload() {
//        assert_eq!(1,2);
//    }
}
