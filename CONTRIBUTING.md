# Contributing to Bevy

Hey, so you're interested in contributing to Bevy!
We're thrilled to have you along as we build!

Check out our community's [Code of Conduct](https://github.com/bevyengine/bevy/blob/main/CODE_OF_CONDUCT.md) and feel free to say hi on [Discord](https://discord.gg/bevy) if you'd like.
It's a nice place to chat about priorities, ask quick questions and get to know the other contributors and users in a less formal setting.

## Getting oriented

Bevy, like any from-scratch game engine, is a large project!
It can be a bit overwhelming to start, so here's the bird's-eye view.

The main [Bevy engine org](https://github.com/bevyengine) has 4 important repos:

1. [`bevy`](https://github.com/bevyengine/bevy) where the engine itself lives, and the bulk of development work occurs.
2. [`bevy-website`](https://github.com/bevyengine/bevy-website) where the [official website](https://bevyengine.org/), release notes and Bevy book are hosted, created using the Zola static site generator for Rust.
3. [`awesome-bevy`](https://github.com/bevyengine/awesome-bevy) is a central home for community content: tutorials, tools, templates, showcases and crates! Make a PR if you want to showcase your stuff there!
4. [`rfcs`](https://github.com/bevyengine/rfcs) a place for informal but detailed discussion and design work for elaborate features and revamps.

The `bevy` repo itself contains many smaller subcrates, each of which can be downloaded on their own, and freely replaced to enable a modular architecture.
Of particular interest, [`bevy_ecs`] is fully functional as a stand-alone ECS, which can be very valuable if you're looking to integrate it with other game engines or use it for non-game executables.

[`bevy_app`] and [`bevy_tasks`] are also worth calling out separately: the former serves as a nice framework to handle various data-piping needs, while the latter is our own lightweight custom async library.

## What we're trying to build

Bevy is intended as an **accessible**, **Rust-first**, **commercially-viable**, **free and open source** game engine.

We prioritize:

* rapid experimentation over API stability
* a consistent vision over a democratic decision making process
* accessible, lightweight workflows over bureaucratic standardization
* a focus on our next goal over immediately integrating major new community-contributed features
* end-user ergonomics over implementation simplicity
* modularity over deep cross-crate integration
* supporting a thriving, easily integrated ecosystem over cramming every feature into the core engine
* control over our code over saving work by reusing existing software
* an ergonomic Rust workflow over a first-party scripting language
* thoughtful public interfaces over maximal configurability
* welcoming contributions over insisting on existing skills and knowledge
* an inclusive environment over avoiding conflict at any cost

## How we're organized

@cart is, for now, our singular Benevolent Dictator and project lead.
He makes the final decision on both design and code changes within Bevy in order to ensure a coherent vision and consistent quality of code.
In practice, @cart serves as a shockingly accountable dictator: open to new ideas and to changing his mind in the face of compelling arguments or community consensus.
Check out the next section for details on how this plays out.

[Bevy org members](https://github.com/orgs/bevyengine/people) are contributors who help keep our repos tidy; they can label and close issues and PRs but do not have merge rights or any special authority within the community.
The bar for trust here is low due to the janitorial nature of the role; feel free to message @cart on GitHub or Discord after you've made a few contributions if you'd like to help out.

## How we work together

Making a game engine is a huge project, but at the moment we only have one paid contributor, @cart (go [donate!](https://github.com/sponsors/cart)).
While we have *many* active contributors (welcome aboard!), herding all of these cats in a predictable way is challenging.

Bevy releases are intended to be spaced 6-8 weeks apart and tend to target one or two major features, led by @cart.
[Once those features are complete](https://github.com/bevyengine/bevy/blob/main/docs/release_checklist.md),
we work to fix any `high-impact` or `regression` tagged issues that we can,
write up our release notes and migration guide, and then announce the next Bevy version to the world!

You can see what we're planning by following along at the [Bevy roadmap](https://github.com/bevyengine/bevy/projects/1).
If you'd like an up-to-the-minute update on progress, feel free to ask on Discord.

But as you may have guessed, that's not *all* that happens.
Simple changes have a simple process:

1. A community member creates an issue or opens a pull request to fix an issue or add simple functionality.
2. Other communities review and comment on the work in an ad-hoc fashion.
3. Once they're content with the quality of the work (code quality, documentation, approach, need for functionality), they individually approve the work.
4. After consensus has been reached (typically two approvals from the community or one for extremely simple changes) and CI passes, the `ready-for-cart` label is added.
5. When @cart has a good opportunity to pause from his implementation work, he performs a final code review on these pull requests and then presses the Big Merge Button (actually, he types `bors r+` to make sure we don't break `main` by accident).

Individual contributors can and do lead major new features and reworks that have caught their interest as well:
these are merged in once they're ready and released as part of the latest version.

Complex changes like this tend to go through the following lifecycle:

1. A need or opportunity is identified in our own projects or by discussing with a user who's asked for help on Discord, reddit or Stack Overflow.
2. An issue is made, laying out the general problem.
3. As needed, this is discussed further on that issue thread, in cross-linked GitHub discussion threads or on Discord in the Engine Development channels.
4. A draft pull request is started, or an RFC is made to solidify a design.
As discussed in the [RFC repo](https://github.com/bevyengine/rfcs), complex features need RFCs, but these can be submitted before or after prototyping work has been started.
5. The community as a whole helps improve the PR and RFC, leaving comments, making suggestions and submitting pull requests to the original branch.
6. Like above, community members approve the PR, add the `ready-for-cart` label and then a final review occurs before merging.

## How you can help

If you've made it to this page, you're probably already convinced that Bevy is a project you'd like to see thrive.
But how can *you* help?

No matter your experience level with Bevy or Rust or your level of commitment, there are ways to meaningfully contribute.
Take a look at the sections that follow to pick a route (or five) that appeal to you.

If you ever find yourself at a loss for what to do, or in need of mentorship or advice on how to contribute to Bevy, feel free to ask in Discord and one of our more experienced community members will be happy to help.

### Battle-testing Bevy

Ultimately, Bevy is a tool that's designed to help people make cool games.
By doing so, you can help us catch bugs, prioritize new features, polish off the rough edges and promote the project.

If you need help, don't hesitate to ask for help on [Discord](https://discord.gg/bevy), [GitHub Discussions](https://github.com/bevyengine/bevy/discussions), [reddit](https://www.reddit.com/r/bevy) or [StackOverflow](https://stackoverflow.com/questions/tagged/bevy).

When you think you've found a bug, some missing documentation or a feature that would help you make better games, please [file an issue](https://github.com/bevyengine/bevy/issues/new/choose) on the main `bevy` repo.
The templates are great, and high-quality issues really do help us!
Do your best to search for duplicate issues, but if you're unsure, open a new issue and link to other related issues within.

And once you've made something that you're proud of, feel free to drop a link, video or screenshot in `#showcase` on Discord!
If you release a game on [itch.io](https://itch.io/games/tag-bevy) we'd be thrilled if you tagged it with `bevy`.

### Teaching others

Bevy is still very young, and light on documentation, tutorials and accumulated expertise.
By teaching others and helping them with their issues, you can learn the engine yourself and make our community better!

Some of the best ways to do this are:

* answering questions on [Discord](https://discord.gg/bevy), [GitHub Discussions](https://github.com/bevyengine/bevy/discussions), [reddit](https://www.reddit.com/r/bevy) or [StackOverflow](https://stackoverflow.com/questions/tagged/bevy).
* writing tutorials, guides and other informal documentation and sharing it on [awesome-bevy](https://github.com/bevyengine/awesome-bevy)
* streaming, writing blog posts about creating your game or creating videos. Share these in the `#devlogs` channel on Discord!

### Writing plugins

If you're interested in contributing to the community ecosystem in a way that doesn't make sense as part of the core engine,
feel free to write a plugin or crate for Bevy!

Any useful, reusable and non-trivial pieces of functionality that you've made serves as a good candidate for a plugin.
If it's closer to a snippet or design pattern, you may want to share it with the community on Discord, Reddit or GitHub Discussions instead.

Check out our [plugin guidelines](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md) for helpful tips and norms.

### Fixing bugs

Bugs in Bevy (or our website / books) are filed on the issue tracker corresponding to where they're found [], using the `bug` label.

If you're looking for an easy place to start, take a look at the `good-first-issue` tag, and feel free to ask questions on the issue in question or on Discord.
You don't need anyone's permission to try fixing a bug or adding a simple feature, but stating that you'd like to tackle an issue can be helpful to avoid duplicated work.

When you make a pull request that fixes an issue, include a line that says `Fixes #X` (or "Closes"), where `X` is the issue number.
This will cause the issue in question to be closed when your PR is merged.

General improvements to code quality are also welcome!
Bevy can always be safer, better tested, and more idiomatic.

### Writing docs

Like every other large, rapidly developing open source library you've ever used, Bevy's documentation could stand to be improved.
This is incredibly valuable, easily distributed work, but requires a bit of guidance:

* inaccurate documentation is worse than no documentation: prioritize fixing broken docs
* Bevy is remarkable unstable: before tackling a new major documentation project, check in with the community about the stability of that feature and upcoming plans to save yourself heartache
* code documentation (doc examples and in the examples folder) is easier to maintain because the compiler will tell us when it breaks
* inline documentation should be fairly direct, minimal and technical
* the Bevy book is hosted on the `bevy-website` repo and targeted towards beginners who are just getting to know Bevy (and perhaps Rust!)
* RFCs are not documentation: they serve as a record of accepted decisions

[docs.rs](https://docs.rs/bevy) is built out of the last release's documentation, which is written right beside the adjacent code.
To view the current docs on `main` before you contribute, clone the `bevy` repo, and run `cargo doc --open`.

### Writing examples

Most [examples in Bevy](https://github.com/bevyengine/bevy/tree/main/examples) aim to clearly demonstrate a single feature, group of closely related small features, or show how to accomplish a particular task (such as asset loading, creating a custom shader or testing of your app).
In rare cases, creating new "game" examples is justified to demonstrate new features
that open a complex class of functionality in a way that's hard to showcase in isolation or requires additional integration testing.

Examples in Bevy should be:

1. **Working.** They must compile and run, and should fail in obvious ways.
2. **Clear.** They must use descriptive variable names, have reasonable code-quality, be formatted, and be appropriately commented.
3. **Relevant.** They should use game-relevant fluff and explain why what they're demonstrating is useful.
4. **Minimal.** They should be no larger or more complex than is needed to meet their other goals.
This reduces maintenance burden and improves clarity when used as a reference.

When you add a new example, be sure to update `examples/README.md` with the new example.
Use a generous sprinkling of keywords in your description: these are commonly used to locate a specific example using CTRL+F.

More complex demonstrations of functionality are also welcome, but for now belong in community tutorials or template games.

Check out [awesome-bevy](https://github.com/bevyengine/awesome-bevy) for a place to put your tutorials, tools, templates and crates!

### Reviewing others work

With the sheer volume of activity in Bevy's community, reviewing others work and helping it improve is one of the most valuable things you can do.
You don't need to be an Elder Rustacean to be useful here: anyone can catch issues of missing tests, unclear docs, logic errors or so on.
If you have unusual skills (e.g. advanced familiarity with `unsafe` code, rendering knowledge or web development experience) or personal experience with a problem, try to prioritize those areas to ensure we can get appropriate expertise where we need it.

Focus on giving constructive, actionable feedback that results in real improvements to code quality or end-user experience.
If you don't understand why an approach was taken, please ask!

Small changes work well as comments or in-line suggestions on specific lines of codes.
Larger changes deserve a comment in the main thread, or a pull request to the original author's branch (but mention that you've made one).
When in doubt about a matter of architectural philosophy, refer back to **What we're trying to build** for guidance.

Once you're happy with the work and feel you're reasonably qualified to assess quality in this particular area, leave your `Approved` review on the PR so we can mark it as `ready-for-cart`.

There are a two main places you can check for new work to review:

1. Pull requests on `bevy` and the `bevy-website` repos.
2. [RFCs](https://github.com/bevyengine/rfcs), which need extensive thoughtful community input on their design.

Official focus areas and work done by @cart go through this review process as well.
Not even our project lead is exempt from reviews and RFCs!
By giving feedback on this work (and related supporting work), you can help us make sure our releases are both high-quality and timely.

Finally, if nothing brings you more satisfaction than seeing every last issue tagged and all resolved issues closed, feel free to message @cart for a Bevy org role to help us keep things tidy.
As discussed in **How we're organized**, this is not intended to have a high bar.

### Contributing your own ideas

As discussed in **How we work together** Bevy is actively open to new ideas and serious contributions from outside community members.
If you're new to Bevy, here's the workflow we use:

1. Fork the `bevyengine/bevy` repository on GitHub, you'll need to create a GitHub account if you don't have one already.
2. Make your changes in a local clone of your fork, typically in its own new branch.
3. For a higher chance of CI passing the first time, consider locally running `cargo run -p ci`. You can run the commands manually:
    1. `cargo fmt --all -- --check` (remove `--check` to let the command fix found problems)
    2. `cargo clippy --workspace --all-targets --all-features -- -D warnings -A clippy::type_complexity`
    3. `cargo test --all-targets --workspace`
4. Push your changes to your fork and open a Pull Request.
5. If you're a first time contributor to this repo, @cart [will need to manually trigger CI for your PR](https://github.blog/changelog/2021-04-22-github-actions-maintainers-must-approve-first-time-contributor-workflow-runs/). Feel free to ping him for this.
6. Respond to any CI failures or review feedback.

If you end up creating a new crate to the `bevy` repo:

1. Add a "Bevy Contributors <bevyengine@gmail.com>" entry in the Author field of `Cargo.toml`.
2. Add an MIT License to match the main `bevy` crate.
3. Add the new crate to the ./tools./publish.sh file.

When contributing, please:

* try to loosely follow the workflow in **How we work together**
* explain what you're doing and why
* document new code with doc comments
* include clear, simple tests
* add or improve the examples for new functionality
* break work into digestible chunks
* ask for any help that you need!

Your first PR will be merged in no time!

No matter how you're helping: thanks for contributing to Bevy!
