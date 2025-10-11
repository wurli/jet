# Assert `Position` `character` fields as UTF-16 based

> <https://github.com/posit-dev/ark/pull/209>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

A major step towards reviving https://github.com/posit-dev/amalthea/pull/83
Requires https://github.com/posit-dev/positron/pull/2093 (you have to `yarn` with it)
Branched from https://github.com/posit-dev/amalthea/pull/211
Addresses https://github.com/posit-dev/positron/issues/2100 (showing we do in fact have a bug currently)

The short form is:
- To implement pull request diagnostics (i.e. refresh diagnostics after executing console input), we need to update `vscode-languageclient` in positron-r first (we will eventually have to do this to stay up to date anyways)
- When you do that, you automatically pull in a new check that REQUIRES the LSP `Server` to support UTF-16 `Position`s (in particular, the `character` column offset field). This is the ONLY encoding that the vs code LSP `Client` supports. I actually believe that it was sending over UTF-16 before, and we had a bug (https://github.com/posit-dev/positron/issues/2100). The change is that it actually errors now, rather than being silent.
- The text document itself (i.e. `Rope` and tree-sitter) is in UTF-8, only the `Position`s we get from / send to the client are in UTF-16.
- So we need a way to convert from LSP UTF-16 `Position`s to tree-sitter UTF-8 `Point`s. That is what this PR implements, trying to be as efficient as I could make it without being an encoding expert.

To implement this we have two new helpers, `convert_position_to_point()` and `convert_point_to_position()`, which handle the details. The `encoding.rs` file holds them, along with more docs about this change. The rest of this PR is basically adapting the LSP codebase to use those helpers, so doesn't need an in depth review.

I tested this locally by inserting a whole bunch of emojis into a document, removing them, inserting some more, and looking at the diagnostics that popped up related to them to ensure the locations of the yellow / red squiggles made sense, and looking at the Output pane to ensure we dont see any errors / panics.

We are not the only ones to have to deal with this.
https://github.com/microsoft/vscode-languageserver-node/issues/1224

Commits that added the UTF-16 error / `PositionEncodingKind` proposal:
https://github.com/microsoft/vscode-languageserver-node/commit/79c2eb195fd90cf10f99e3f74dda0858f11074ff
https://github.com/microsoft/vscode-languageserver-node/commit/1ab1a69884667a71a2d2e04bf42c04622d460044

## @DavisVaughan at 2024-01-23T15:56:16Z

For use when we merge the rope PR

```rust
pub fn convert_position_to_point2(x: &Rope, position: Position) -> Point {
    let line = position.line as usize;
    let character = position.character as usize;

    let character = with_line(x, line, character, convert_character_from_utf16_to_utf8);

    Point::new(line, character)
}

pub fn convert_point_to_position2(x: &Rope, point: Point) -> Position {
    let line = point.row;
    let character = point.column;

    let character = with_line(x, line, character, convert_character_from_utf8_to_utf16);

    let line = line as u32;
    let character = character as u32;

    Position::new(line, character)
}

fn with_line<F>(x: &Rope, line: usize, character: usize, f: F) -> usize
where
    F: FnOnce(&str, usize) -> usize,
{
    // Empty documents come through as an empty string, which looks like 0 lines (TODO: Confirm this?)
    if x.len_lines() == 0 {
        if line != 0 || character != 0 {
            log::error!("Document is empty, but using position: ({line}, {character})");
        }
        return 0;
    }

    let Some(x) = x.get_line(line) else {
        let n = x.len_lines();
        let x = x.to_string();
        let line = line + 1;
        log::error!("Requesting line {line} but only {n} lines exist. Document: '{x}'.");
        return 0;
    };

    // If the line is fully contained in a single chunk (likely is), use free conversion to `&str`
    if let Some(x) = x.as_str() {
        return f(x, character);
    }

    // Otherwise, use ever so slightly more expensive String materialization of the
    // line spread across chunks
    let x = x.to_string();
    let x = x.as_str();

    f(x, character)
}
```
