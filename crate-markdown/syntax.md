# syntax

tl;dr mostly matches github flavored markdown with some extensions

- any character normally used for formatting can be escaped with a backslash (`\\`) to interpret that character literally

## block

- newlines are hard breaks. empty lines count as empty paragraph nodes.
  - eg. a blockquote doesnt need to be followed by an empty line
- headers are `# ` followed by content. the number of `#` is the heading level. you can use up to 6 `#`.
- codeblocks are at least three backticks followed by optional language and a newline, some code, then the *same number* of backticks to close the block
- blockquotes are `> ` followed by content. blockquotes can span multiple lines if each line is prefixed with a `> `

### lists

- ordered lists are lines prefixed with `1. ` where `1` can be any number
- unordered lists are lines prefixed with `- `
- task lists are lines prefixed with `- [x]` where the `x` may be a ` `
- lists may span multiple lines if subsequent lines are indented to the end of the list prefix
- lists contain block content and thus may be indented

example:

```
123. list item
     multiple lines
     456. nested
```

### tables

- table content is a set of blocks
- table row lines are a sequences of text starting with, ending with, and separated by pipes `|`
- table rows are one or more table row lines separated with newlines.
  - each line in a table row is be split by `|`
  - join segments with the same offset in each split line with newlines to recover the original block
- table alignment is are one or more `-` characters, optionally starting and/or ending with `:`
  - `---` automatic alignment
  - `:---` left aligned
  - `:---` right aligned
  - `:---:` center aligned
- table alignment row is a table row where each cell contains a table alignment
- table is a table row, newline, table alignment row, newline, then zero or more table rows
- tables cannot be nested

## inline

- `*` for bold
- `**` for emphasis
- `\`` for code
- `~~text here~~` for strikethrough
- `||text here||` for spoilers
- `<@uuid-here>` for user mentions
- `<&uuid-here>` for role mentions
- `<#uuid-here>` for channel mentions
- `<:name:uuid-here>` for custom emoji
- `<a:name:uuid-here>` for animated custom emoji
- `https://example.com` urls are autolinked

## notes

- angle brackets for links are stripped and ignored
- since all newlines are hard breaks, theres no way to insert a soft line break
- when using `***` for emphasis + strong, you can use either `*` or `**` to end only emphasis or strong respectively
- blank paragraphs *may* be collapsed or removed, but also may be preserved
- whitespace before/after/inside lines *may* be collapsed, but also may be preserved

### unsupported syntax

- underscores (`_`) don't do anything
- indented code blocks
- setext headings (lines with `===` or `---` underneath)
- horizontal rules (`---`)
- image links
- raw html
- html entities (`&amp;`, `&#42`); use the corresponding unicode char instead
- unordered list items prefixed with `*` or `+`
- link references
