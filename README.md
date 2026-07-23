Premade things that I cloned

ratatui's own examples (ratatui/examples/apps/) — the ones that i used:

user_input — input line with cursor, insert/delete, submit. This is your prompt.
list / scrollbar — scrollback and the ScrollbarState pattern.
tabs — buffer switching bar.
chart/demo2 — ignore.

Widget crates:

tui-textarea (rhysd) — drop-in input widget, handles cursor/yank/kill-line/multiline. Its single_line.rs and popup_placeholder.rs examples are basically an IRC prompt already.
tui-scrollview (joshka) — if you don't want to hand-roll wrap+scroll math.
tui-popup — for /help, server list.
tui-logger — dump raw IRC wire traffic into a debug pane. Very useful while building the protocol layer.

Templates: cargo generate ratatui/templates → the simple-async template is exactly the shape you want (tokio + crossterm EventStream + action channel).

Layout reference: tiny's libtiny_tui — not ratatui, but the message-line rendering, nick coloring by hash, and consecutive-join/part collapsing are the ideas worth stealing.

Scaffold

Rather than stitch those together, here's a working one — compiles clean on ratatui 0.30.2 / crossterm 0.29, edition 2024.

It has: tab bar with unread + highlight states, wrapped scrollback with a right-aligned nick gutter, hash-based nick colors, nick list pane with op/voice sorting, input line with cursor + Ctrl-U, tab-completion, Alt-1..9 / Ctrl-N / Ctrl-P buffer switching, PageUp/Down scroll, and highlight detection on your nick.

The Event enum in app.rs is the seam. A fake server task feeds it on a tokio::mpsc channel; tonight you drive the whole UI off that. When you do the protocol layer, delete fake_server and have your socket task send the same enum — nothing in ui.rs changes.

Things deliberately left rough so you have something to do: wrapping counts chars not display width (swap in unicode-width for CJK/emoji), no /command handling (submit() returns the raw line, cmd_rx currently drains it), no mIRC color-code parsing, and render_lines rebuilds every frame — fine at 5k messages, worth caching later.