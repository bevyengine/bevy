# Rustdoc Postprocessor

We want to adjust rustdoc's html output to make it more obvious
which types are `Component`s, `Plugin`s etc. To do so, this
tool wraps rustdoc and modifies its output by adding relevant tags
to the top of a type's doc page.

The wrapper is called by passing
`--config "build.rustdoc = \"tools/rustdoc-wrapper/rustdoc.sh\""`
to cargo doc.
