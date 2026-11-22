//! Finding web links in a task's text.

const SCHEMES: [&str; 2] = ["https://", "http://"];

/// Every http or https link in `text`, with the byte it starts at.
pub fn find(text: &str) -> Vec<(usize, &str)> {
    let mut links = Vec::new();
    let mut from = 0;
    while from < text.len() {
        let Some(start) = next_start(text, from) else {
            break;
        };
        let end = text[start..]
            .find(|c: char| c.is_whitespace() || matches!(c, '<' | '"'))
            .map_or(text.len(), |len| start + len);
        let link = trimmed(&text[start..end]);
        if link
            .find("://")
            .is_some_and(|scheme| link.len() > scheme + 3)
        {
            links.push((start, link));
        }
        from = end;
    }
    links
}

/// The link under `column`, counted in characters, or else the first on
/// `line`.
pub fn at(line: &str, column: usize) -> Option<&str> {
    let byte = line
        .char_indices()
        .nth(column)
        .map_or(line.len(), |(byte, _)| byte);
    let links = find(line);
    links
        .iter()
        .find(|(start, link)| (*start..start + link.len()).contains(&byte))
        .or(links.first())
        .map(|(_, link)| *link)
}

/// The first link in any of `texts`.
pub fn first<'a>(texts: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    texts
        .into_iter()
        .find_map(|text| find(text).first().map(|(_, link)| *link))
}

fn next_start(text: &str, from: usize) -> Option<usize> {
    (from..text.len()).find(|&i| {
        text.is_char_boundary(i)
            && !text[..i]
                .chars()
                .next_back()
                .is_some_and(char::is_alphanumeric)
            && SCHEMES.iter().any(|scheme| {
                text.get(i..i + scheme.len())
                    .is_some_and(|head| head.eq_ignore_ascii_case(scheme))
            })
    })
}

/// Drops the punctuation a sentence puts after a link, and closing brackets
/// the link did not open.
fn trimmed(mut link: &str) -> &str {
    loop {
        let Some(last) = link.chars().next_back() else {
            return link;
        };
        let unopened = match last {
            ')' => Some('('),
            ']' => Some('['),
            '}' => Some('{'),
            '>' => Some('<'),
            _ => None,
        }
        .is_some_and(|open| link.matches(open).count() < link.matches(last).count());
        if !unopened && !".,;:!?'".contains(last) {
            return link;
        }
        link = &link[..link.len() - last.len_utf8()];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn links(text: &str) -> Vec<&str> {
        find(text).into_iter().map(|(_, link)| link).collect()
    }

    #[test]
    fn links_are_found_among_words() {
        assert_eq!(
            links("See https://example.com/a?b=1&c=2 and http://x.org/"),
            ["https://example.com/a?b=1&c=2", "http://x.org/"]
        );
        assert_eq!(links("HTTPS://Example.com"), ["HTTPS://Example.com"]);
        assert_eq!(links("no links, just http and https"), Vec::<&str>::new());
    }

    #[test]
    fn only_web_links_count() {
        assert_eq!(
            links("file:///etc/passwd ftp://x mailto:a@b.c"),
            Vec::<&str>::new()
        );
        assert_eq!(links("xhttps://example.com"), Vec::<&str>::new());
        assert_eq!(links("https:// alone"), Vec::<&str>::new());
        assert_eq!(links("http://x"), ["http://x"]);
    }

    #[test]
    fn sentence_punctuation_is_not_part_of_the_link() {
        assert_eq!(links("Read https://example.com."), ["https://example.com"]);
        assert_eq!(
            links("(see https://example.com/page)"),
            ["https://example.com/page"]
        );
        assert_eq!(
            links("https://en.wikipedia.org/wiki/Rust_(programming_language),"),
            ["https://en.wikipedia.org/wiki/Rust_(programming_language)"]
        );
        assert_eq!(links("<https://example.com>"), ["https://example.com"]);
        assert_eq!(links("\"https://example.com\""), ["https://example.com"]);
    }

    #[test]
    fn the_link_under_the_cursor_wins_over_the_first() {
        let line = "a https://one.com b https://two.com";

        assert_eq!(at(line, 25), Some("https://two.com"));
        assert_eq!(at(line, 2), Some("https://one.com"));
        assert_eq!(at(line, 0), Some("https://one.com"));
        assert_eq!(at("nothing here", 3), None);
    }

    #[test]
    fn the_cursor_counts_characters_not_bytes() {
        let line = "ææ https://one.com ø https://two.com";

        assert_eq!(at(line, 22), Some("https://two.com"));
    }

    #[test]
    fn the_first_link_of_several_texts() {
        assert_eq!(
            first(["Title", "notes https://a.com", "https://b.com"]),
            Some("https://a.com")
        );
        assert_eq!(first(["none", ""]), None);
    }
}
