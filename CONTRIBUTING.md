# Contributing to Bevy

Hey, so you're interested in contributing to Bevy!
Feel free to pitch in on whatever interests you and we'll be happy to help you contribute.
Ultimately @cart has final say on which changes are merged, but you're welcome to try and convince him.

Check out our community's [Code of Conduct](https://github.com/bevyengine/bevy/blob/main/CODE_OF_CONDUCT.md) and feel free to say hi on [Discord](https://discord.gg/bevy) if you'd like.
It's a nice place to chat about Bevy development, ask questions, and get to know the other contributors and users in a less formal setting.

Read on if you're looking for:

* the high-level design goals of Bevy
* conventions and informal practices we follow when developing Bevy
* general advice on good open source collaboration practices
* concrete ways you can help us, no matter your background or skill level

We're thrilled to have you along as we build!

## Getting oriented

Bevy, like any general-purpose game engine, is a large project!
It can be a bit overwhelming to start, so here's the bird's-eye view.

The [Bevy Engine Organization](https://github.com/bevyengine) has 4 primary repos:

1. [**`bevy`**](https://github.com/bevyengine/bevy): This is where the engine itself lives. The bulk of development work occurs here.
2. [**`bevy-website`**](https://github.com/bevyengine/bevy-website): Where the [official website](https://bevyengine.org/), release notes, Bevy Book, and Bevy Assets are hosted. It is created using the Zola static site generator.
3. [**`bevy-assets`**](https://github.com/bevyengine/bevy-assets): A collection of community-made tutorials, plugins, crates, games, and tools! Make a PR if you want to showcase your projects there!
4. [**`rfcs`**](https://github.com/bevyengine/rfcs): A place to collaboratively build and reach consensus on designs for large or controversial features.

The `bevy` repo itself contains many smaller subcrates. Most of them can be used by themselves and many of them can be modularly replaced. This enables developers to pick and choose the parts of Bevy that they want to use.

Some crates of interest:

* [**`bevy_ecs`**](./crates/bevy_ecs): The core data model for Bevy. Most Bevy features are implemented on top of it. It is also fully functional as a stand-alone ECS, which can be very valuable if you're looking to integrate it with other game engines or use it for non-game executables.
* [**`bevy_app`**](./crates/bevy_app): The api used to define Bevy Plugins and compose them together into Bevy Apps.
* [**`bevy_tasks`**](./crates/bevy_tasks): Our light-weight async executor. This drives most async and parallel code in Bevy.
* [**`bevy_render`**](./crates/bevy_render): Our core renderer API. It handles interaction with the GPU, such as the creation of Meshes, Textures, and Shaders. It also exposes a modular Render Graph for composing render pipelines. All 2D and 3D render features are implemented on top of this crate.

## What we're trying to build

Bevy is a completely free and open source game engine built in Rust. It currently has the following design goals:

* **Capable**: Offer a complete 2D and 3D feature set
* **Simple**: Easy for newbies to pick up, but infinitely flexible for power users
* **Data Focused**: Data-oriented architecture using the Entity Component System paradigm
* **Modular**: Use only what you need. Replace what you don't like
* **Fast**: App logic should run quickly, and when possible, in parallel
* **Productive**: Changes should compile quickly ... waiting isn't fun

Bevy also currently has the following "development process" goals:

* **Rapid experimentation over API stability**: We need the freedom to experiment and iterate in order to build the best engine we can. This will change over time as APIs prove their staying power.
* **Consistent vision**: The engine needs to feel consistent and cohesive. This takes precedent over democratic and/or decentralized processes. See [*How we're organized*](#how-were-organized) for more details.
* **Flexibility over bureaucracy**: Developers should feel productive and unencumbered by development processes.
* **Focus**: The Bevy Org should focus on building a small number of features excellently over merging every new community-contributed feature quickly. Sometimes this means pull requests will sit unmerged for a long time. This is the price of focus and we are willing to pay it. Fortunately Bevy is modular to its core. 3rd party plugins are a great way to work around this policy.
* **User-facing API ergonomics come first**: Solid user experience should receive significant focus and investment. It should rarely be compromised in the interest of internal implementation details.  
* **Modularity over deep integration**: Individual crates and features should be "pluggable" whenever possible. Don't tie crates, features, or types together that don't need to be.
* **Don't merge everything ... don't merge too early**: Every feature we add increases maintenance burden and compile times. Only merge features that are "generally" useful. Don't merge major changes or new features unless we have relative consensus that the design is correct _and_ that we have the developer capacity to support it. When possible, make a 3rd party Plugin / crate first, then consider merging once the API has been tested in the wild. Bevy's modular structure means that the only difference between "official engine features" and "third party plugins" is our endorsement and the repo the code lives in. We should take advantage of that whenever possible.
* **Control and consistency over 3rd party code reuse**: Only add a dependency if it is _absolutely_ necessary. Every dependency we add decreases our autonomy and consistency. Dependencies also have the potential to increase compile times and risk pulling in sub-dependencies we don't want / need.
* **Don't re-invent every wheel**: As a counter to the previous point, don't re-invent everything at all costs. If there is a crate in the Rust ecosystem that is the "de-facto" standard (ex: wgpu, winit, cpal), we should heavily consider using it. Bevy should be a positive force in the ecosystem. We should drive the improvements we need into these core ecosystem crates.
* **Rust-first**: Engine and user-facing code should optimize and encourage Rust-only workflows. Adding additional languages increases internal complexity, fractures the Bevy ecosystem, and makes it harder for users to understand the engine. Never compromise a Rust interface in the interest of compatibility with other languages.
* **Thoughtful public interfaces over maximal configurability**: Symbols and apis should be private by default. Every public API should be thoughtfully and consistently designed. Don't expose unnecessary internal implementation details. Don't allow users to "shoot themselves in the foot". Favor one "happy path" api over multiple apis for different use cases.
* **Welcome new contributors**: Invest in new contributors. Help them fill knowledge and skill gaps. Don't ever gatekeep Bevy development according to notions of required skills or credentials. Help new developers find their niche.
* **Civil discourse**: We need to collectively discuss ideas and the best ideas _should_ win. But conversations need to remain respectful at all times. Remember that we're all in this together. Always follow our [Code of Conduct](https://github.com/bevyengine/bevy/blob/main/CODE_OF_CONDUCT.md).
* **Test what you need to**: Write useful tests. Don't write tests that aren't useful. We _generally_ aren't strict about unit testing every line of code. We don't want you to waste your time. But at the same time:
  * Most new features should have at least one minimal [example](https://github.com/bevyengine/bevy/tree/main/examples). These also serve as simple integration tests, as they are run as part of our CI process.
  * The more complex or "core" a feature is, the more strict we are about unit tests. Use your best judgement here. We will let you know if your pull request needs more tests. We use [Rust's built in testing framework](https://doc.rust-lang.org/book/ch11-01-writing-tests.html).

## How we're organized

@cart is, for now, our singular Benevolent Dictator and project lead.
He makes the final decision on both design and code changes within Bevy in order to ensure a coherent vision and consistent quality of code.

In practice, @cart serves as a shockingly accountable dictator: open to new ideas and to changing his mind in the face of compelling arguments or community consensus.
Check out the next section for details on how this plays out.

[Bevy Org members](https://github.com/orgs/bevyengine/people) are contributors who have:

1. Have actively engaged with Bevy development
2. Have demonstrated themselves to be polite and welcoming representatives of the project with an understanding of our goals and direction.
3. Have asked to join the Bevy Org. Reach out to @cart on Discord or email us at bevyengine@gmail.com if you are interested. Everyone is welcome to do this. We generally accept membership requests, so don't hesitate if you are interested!

Some Bevy Org members are also [Triage Team](https://github.com/orgs/bevyengine/teams/triage-team) members. These people can label and close issues and PRs but do not have merge rights or any special authority within the community. Existing Bevy Engine Org members can [automatically request membership](https://github.com/orgs/bevyengine/teams/triage-team/members). Once again, if you are interested don't hesitate to apply. We generally accept membership requests.

We heavily limit who has merge rights within the org because this requires a large amount of trust when it comes to ethics, technical ability, and ability to enforce consistent project direction. Currently, only @cart can merge every class of change. @mockersf is allowed to merge small "uncontroversial" changes, provided these changes have at least two approvals. Same goes for @alice-i-cecile and doc related changes. If there is an emergency that needs a quick resolution and @cart is not around, both @mockersf and @alice-i-cecile are allowed to merge changes that resolve the emergency.

## How we work together

Making a game engine is a huge project and facilitating collaboration is a lot of work. At the moment @cart is our only paid contributor, so [go sponsor him!](https://github.com/sponsors/cart)
We track issues and pull requests that must be included in releases using [Milestones](https://github.com/bevyengine/bevy/milestones).

You can see what we're planning by following along at the [Bevy Roadmap](https://github.com/bevyengine/bevy/projects/1).
If you'd like an up-to-the-minute look at our progress on a specific project, feel free to ask on Discord.

### Making changes to Bevy

Most changes don't require much "process". If your change is relatively straightforward, just do the following:

1. A community member (that's you!) creates one of the following:
    * [Discussion](https://github.com/bevyengine/bevy/discussions): An informal discussion with the community. This is the place to start if you want to propose a feature or specific implementation.
    * [Issue](https://github.com/bevyengine/bevy/issues): A formal way for us to track a bug or feature. Please look for duplicates before opening a new issue and consider starting with a Discussion.
    * [Pull Request](https://github.com/bevyengine/bevy/pulls) (or PR for short): A request to merge code changes. This starts our "review process". You are welcome to start with a pull request, but consider starting with an Issue or Discussion for larger changes (or if you aren't certain about a design). We don't want anyone to waste their time on code that didn't have a chance to be merged! But conversely, sometimes PRs are the most efficient way to propose a change. Just use your own judgement here.
2. Other community members review and comment in an ad-hoc fashion. Active subject matter experts may be pulled into a thread using `@mentions`. If your PR has been quiet for a while and is ready for review, feel free to leave a message to "bump" the thread, or bring it up on Discord in an appropriate engine development channel.
3. Once they're content with the pull request (design, code quality, documentation, tests), individual reviewers leave "Approved" reviews.
4. After consensus has been reached (typically two approvals from the community or one for extremely simple changes) and CI passes, the [S-Ready-For-Final-Review](https://github.com/bevyengine/bevy/issues?q=is%3Aopen+is%3Aissue+label%3AS-Ready-For-Final-Review) label is added.
5. When they find time, [someone with merge rights](#how-were-organized) performs a final code review and merges the PR using [Bors](https://bors.tech/) by typing `bors r+`.

### Complex changes

Individual contributors often lead major new features and reworks. However these changes require more design work and scrutiny. Complex changes like this tend to go through the following lifecycle:

1. A need or opportunity is identified and an issue is made, laying out the general problem.
2. As needed, this is discussed further on that issue thread, in cross-linked GitHub discussion threads, or on Discord in the Engine Development channels.
3. Either a Draft Pull Request or an RFC is made. As discussed in the [RFC repo](https://github.com/bevyengine/rfcs), complex features need RFCs, but these can be submitted before or after prototyping work has been started.
4. The community as a whole helps improve the Draft PR and/or RFC, leaving comments, making suggestions, and submitting pull requests to the original branch.
5. Once the RFC is merged and/or the Draft Pull Request is transitioned out of draft mode, the [normal change process outlined in the previous section](#making-changes-to-bevy) can begin.

## How you can help

If you've made it to this page, you're probably already convinced that Bevy is a project you'd like to see thrive.
But how can *you* help?

No matter your experience level with Bevy or Rust or your level of commitment, there are ways to meaningfully contribute.
Take a look at the sections that follow to pick a route (or five) that appeal to you.

If you ever find yourself at a loss for what to do, or in need of mentorship or advice on how to contribute to Bevy, feel free to ask in Discord and one of our more experienced community members will be happy to help.

### Battle-testing Bevy

Ultimately, Bevy is a tool that's designed to help people make cool games.
By using Bevy, you can help us catch bugs, prioritize new features, polish off the rough edges, and promote the project.

If you need help, don't hesitate to ask for help on [GitHub Discussions](https://github.com/bevyengine/bevy/discussions), [Discord](https://discord.gg/bevy), or [reddit](https://www.reddit.com/r/bevy). Generally you should prefer asking questions as Github Discussions as they are more searchable.

When you think you've found a bug, missing documentation, or a feature that would help you make better games, please [file an issue](https://github.com/bevyengine/bevy/issues/new/choose) on the main `bevy` repo.

Do your best to search for duplicate issues, but if you're unsure, open a new issue and link to other related issues on the thread you make.

Once you've made something that you're proud of, feel free to drop a link, video, or screenshot in `#showcase` on Discord!
If you release a game on [itch.io](https://itch.io/games/tag-bevy) we'd be thrilled if you tagged it with `bevy`.

### Teaching others

Bevy is still very young, and light on documentation, tutorials and accumulated expertise.
By helping others with their issues, and teaching them about Bevy, you will naturally learn the engine and codebase in greater depth while also making our community better!

Some of the best ways to do this are:

* Answering questions on [GitHub Discussions](https://github.com/bevyengine/bevy/discussions), [Discord](https://discord.gg/bevy), and [reddit](https://www.reddit.com/r/bevy).
* Writing tutorials, guides, and other informal documentation and sharing them on [Bevy Assets](https://github.com/bevyengine/bevy-assets)
* Streaming, writing blog posts about creating your game, and creating videos. Share these in the `#devlogs` channel on Discord!

### Writing plugins

You can improve Bevy's ecosystem by building your own Bevy Plugins and crates.

Non-trivial, reusable functionality that works well with itself is a good candidate for a plugin.
If it's closer to a snippet or design pattern, you may want to share it with the community on Discord, Reddit, or GitHub Discussions instead.

Check out our [plugin guidelines](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md) for helpful tips and patterns.

### Fixing bugs

Bugs in Bevy (or the associated website / book) are filed on the issue tracker using the [`bug`](https://github.com/bevyengine/bevy/issues?q=is%3Aissue+is%3Aopen+label%3Abug) label.

If you're looking for an easy place to start, take a look at the [`E-Good-First-Issue`](https://github.com/bevyengine/bevy/issues?q=is%3Aopen+is%3Aissue+label%3AE-Good-First-Issue) label, and feel free to ask questions on that issue's thread in question or on Discord.
You don't need anyone's permission to try fixing a bug or adding a simple feature, but stating that you'd like to tackle an issue can be helpful to avoid duplicated work.

When you make a pull request that fixes an issue, include a line that says `Fixes #X` (or "Closes"), where `X` is the issue number.
This will cause the issue in question to be closed when your PR is merged.

General improvements to code quality are also welcome!
Bevy can always be safer, better tested, and more idiomatic.

### Writing docs

Like every other large, rapidly developing open source library you've ever used, Bevy's documentation can always use improvement.
This is incredibly valuable, easily distributed work, but requires a bit of guidance:

* inaccurate documentation is worse than no documentation: prioritize fixing broken docs
* Bevy is remarkable unstable: before tackling a new major documentation project, check in with the community on Discord or GitHub (making an issue about specific missing docs is a great way to plan) about the stability of that feature and upcoming plans to save yourself heartache
* code documentation (doc examples and in the examples folder) is easier to maintain because the compiler will tell us when it breaks
* inline documentation should be technical and to the point. Link relevant examples or other explanations if broader context is useful.
* the Bevy book is hosted on the `bevy-website` repo and targeted towards beginners who are just getting to know Bevy (and perhaps Rust!)
* accepted RFCs are not documentation: they serve only as a record of accepted decisions

[docs.rs](https://docs.rs/bevy) is built from out of the last release's documentation, which is written right in-line directly above the code it documents.
To view the current docs on `main` before you contribute, clone the `bevy` repo, and run `cargo doc --open`.

### Writing examples

Most [examples in Bevy](https://github.com/bevyengine/bevy/tree/main/examples) aim to clearly demonstrate a single feature, group of closely related small features, or show how to accomplish a particular task (such as asset loading, creating a custom shader or testing your app).
In rare cases, creating new "game" examples is justified in order to demonstrate new features
that open a complex class of functionality in a way that's hard to demonstrate in isolation or requires additional integration testing.

Examples in Bevy should be:

1. **Working:** They must compile and run, and any introduced errors in them should be obvious (through tests, simple results or clearly displayed behavior).
2. **Clear:** They must use descriptive variable names, be formatted, and be appropriately commented. Try your best to showcase best practices when it doesn't obscure the point of the example.
3. **Relevant:** They should explain, through comments or variable names, what they do and how this can be useful to a game developer.
4. **Minimal:** They should be no larger or complex than is needed to meet the goals of the example.

When you add a new example, be sure to update `examples/README.md` with the new example and add it to the root `Cargo.toml` file.
Use a generous sprinkling of keywords in your description: these are commonly used to search for a specific example.
See the [example style guide](.github/contributing/example_style_guide.md) to help make sure the style of your example matches what we're already using.

More complex demonstrations of functionality are also welcome, but these should be submitted to [bevy-assets](https://github.com/bevyengine/bevy-assets).

### Reviewing others' work

With the sheer volume of activity in Bevy's community, reviewing others work with the aim of improving it is one of the most valuable things you can do.
You don't need to be an Elder Rustacean to be useful here: anyone can catch missing tests, unclear docs, logic errors, and so on.
If you have specific skills (e.g. advanced familiarity with `unsafe` code, rendering knowledge or web development experience) or personal experience with a problem, try to prioritize those areas to ensure we can get appropriate expertise where we need it.

Focus on giving constructive, actionable feedback that results in real improvements to code quality or end-user experience.
If you don't understand why an approach was taken, please ask!

Provide actual code suggestions when that is helpful. Small changes work well as comments or in-line suggestions on specific lines of codes.
Larger changes deserve a comment in the main thread, or a pull request to the original author's branch (but please mention that you've made one).
When in doubt about a matter of architectural philosophy, refer back to [*What we're trying to build*](#what-were-trying-to-build) for guidance.

Once you're happy with the work and feel you're reasonably qualified to assess quality in this particular area, leave your `Approved` review on the PR.
If you're new to GitHub, check out the [Pull Request Review documentation](https://docs.github.com/en/github/collaborating-with-pull-requests/reviewing-changes-in-pull-requests/about-pull-request-reviews). Anyone can leave reviews ... no special permissions are required!

There are a two main places you can check for things to review:

1. Pull requests on [bevy](https://github.com/bevyengine/bevy/pulls) and the [bevy-website](https://github.com/bevyengine/bevy-website/pulls) repos.
2. [RFCs](https://github.com/bevyengine/rfcs), which need extensive thoughtful community input on their design.

Official focus areas and work done by @cart go through this review process as well.
Not even our project lead is exempt from reviews and RFCs!
By giving feedback on this work (and related supporting work), you can help us make sure our releases are both high-quality and timely.

Finally, if nothing brings you more satisfaction than seeing every last issue labeled and all resolved issues closed, feel free to message @cart for a Bevy org role to help us keep things tidy.
As discussed in [*How we're organized*](#how-were-organized), this role only requires good faith and a basic understanding of our development process.

### Contributing code

Bevy is actively open to code contributions from community members.
If you're new to Bevy, here's the workflow we use:

1. Fork the `bevyengine/bevy` repository on GitHub. You'll need to create a GitHub account if you don't have one already.
2. Make your changes in a local clone of your fork, typically in its own new branch.
   1. Try to split your work into separate commits, each with a distinct purpose. Be particularly mindful of this when responding to reviews so it's easy to see what's changed.
3. To test CI validations locally, run the `cargo run -p ci` command. You can also run sub-commands manually:
    1. `cargo fmt --all -- --check` (remove `--check` to let the command fix found problems)
    2. `cargo clippy --workspace --all-targets --all-features -- -D warnings -A clippy::type_complexity`
    3. `cargo test --all-targets --workspace`
4. When working with Markdown (`.md`) files, Bevy's CI will check markdown files (like this one) using [markdownlint](https://github.com/DavidAnson/markdownlint).
To locally lint your files using the same workflow as our CI:
   1. Install [markdownlint-cli](https://github.com/igorshubovych/markdownlint-cli).
   2. Run `markdownlint -f -c .github/linters/.markdown-lint.yml .` in the root directory of the Bevy project.
5. Push your changes to your fork on Github and open a Pull Request.
6. If your account is new on github, one of the Bevy org members [will need to manually trigger CI for your PR](https://github.blog/changelog/2021-04-22-github-actions-maintainers-must-approve-first-time-contributor-workflow-runs/) using the `bors try` command.
7. Respond to any CI failures or review feedback. While CI failures must be fixed before we can merge your PR, you do not need to *agree* with all feedback from your reviews, merely acknowledge that it was given. If you cannot come to an agreement, leave the thread open and defer to @cart's final judgement.
8. When your PR is ready to merge, @cart will review it and suggest final changes. If those changes are minimal he may even apply them directly to speed up merging.

If you end up adding a new official Bevy crate to the `bevy` repo:

1. Add the new crate to the [./tools/publish.sh](./tools/publish.sh) file.
2. Check if a new cargo feature was added, update [cargo_features.md](https://github.com/bevyengine/bevy/blob/main/docs/cargo_features.md) as needed.

When contributing, please:

* try to loosely follow the workflow in [*How we work together*](#how-we-work-together)
* consult the [style guide](.github/contributing/engine_style_guide.md) to help keep our code base tidy
* explain what you're doing and why
* document new code with doc comments
* include clear, simple tests
* add or improve the examples when adding new user-facing functionality
* break work into digestible chunks
* ask for any help that you need!

Your first PR will be merged in no time!

No matter how you're helping: thanks for contributing to Bevy!
