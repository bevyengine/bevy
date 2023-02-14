# Contributing to Bevy

Hey, so you're interested in contributing to Bevy!
Feel free to pitch in on whatever interests you and we'll be happy to help you contribute.

Check out our community's [Code of Conduct](https://github.com/bevyengine/bevy/blob/main/CODE_OF_CONDUCT.md) and feel free to say hi on [Discord] if you'd like.
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
* **Consistent vision**: The engine needs to feel consistent and cohesive. This takes precedent over democratic and/or decentralized processes. See our [*Bevy Organization doc*](/docs/the_bevy_organization.md) for more details.
* **Flexibility over bureaucracy**: Developers should feel productive and unencumbered by development processes.
* **Focus**: The Bevy Org should focus on building a small number of features excellently over merging every new community-contributed feature quickly. Sometimes this means pull requests will sit unmerged for a long time. This is the price of focus and we are willing to pay it. Fortunately Bevy is modular to its core. 3rd party plugins are a great way to work around this policy.
* **User-facing API ergonomics come first**: Solid user experience should receive significant focus and investment. It should rarely be compromised in the interest of internal implementation details.  
* **Modularity over deep integration**: Individual crates and features should be "pluggable" whenever possible. Don't tie crates, features, or types together that don't need to be.
* **Don't merge everything ... don't merge too early**: Every feature we add increases maintenance burden and compile times. Only merge features that are "generally" useful. Don't merge major changes or new features unless we have relative consensus that the design is correct *and* that we have the developer capacity to support it. When possible, make a 3rd party Plugin / crate first, then consider merging once the API has been tested in the wild. Bevy's modular structure means that the only difference between "official engine features" and "third party plugins" is our endorsement and the repo the code lives in. We should take advantage of that whenever possible.
* **Control and consistency over 3rd party code reuse**: Only add a dependency if it is *absolutely* necessary. Every dependency we add decreases our autonomy and consistency. Dependencies also have the potential to increase compile times and risk pulling in sub-dependencies we don't want / need.
* **Don't re-invent every wheel**: As a counter to the previous point, don't re-invent everything at all costs. If there is a crate in the Rust ecosystem that is the "de-facto" standard (ex: wgpu, winit, cpal), we should heavily consider using it. Bevy should be a positive force in the ecosystem. We should drive the improvements we need into these core ecosystem crates.
* **Rust-first**: Engine and user-facing code should optimize and encourage Rust-only workflows. Adding additional languages increases internal complexity, fractures the Bevy ecosystem, and makes it harder for users to understand the engine. Never compromise a Rust interface in the interest of compatibility with other languages.
* **Thoughtful public interfaces over maximal configurability**: Symbols and apis should be private by default. Every public API should be thoughtfully and consistently designed. Don't expose unnecessary internal implementation details. Don't allow users to "shoot themselves in the foot". Favor one "happy path" api over multiple apis for different use cases.
* **Welcome new contributors**: Invest in new contributors. Help them fill knowledge and skill gaps. Don't ever gatekeep Bevy development according to notions of required skills or credentials. Help new developers find their niche.
* **Civil discourse**: We need to collectively discuss ideas and the best ideas *should* win. But conversations need to remain respectful at all times. Remember that we're all in this together. Always follow our [Code of Conduct](https://github.com/bevyengine/bevy/blob/main/CODE_OF_CONDUCT.md).
* **Test what you need to**: Write useful tests. Don't write tests that aren't useful. We *generally* aren't strict about unit testing every line of code. We don't want you to waste your time. But at the same time:
  * Most new features should have at least one minimal [example](https://github.com/bevyengine/bevy/tree/main/examples). These also serve as simple integration tests, as they are run as part of our CI process.
  * The more complex or "core" a feature is, the more strict we are about unit tests. Use your best judgement here. We will let you know if your pull request needs more tests. We use [Rust's built in testing framework](https://doc.rust-lang.org/book/ch11-01-writing-tests.html).

## The Bevy Organization

The Bevy Organization is the group of people responsible for stewarding the Bevy project. It handles things like merging pull requests, choosing project direction, managing bugs / issues / feature requests, running the Bevy website, controlling access to secrets, defining and enforcing best practices, etc.

Note that you *do not* need to be a member of the Bevy Organization to contribute to Bevy. Community contributors (this means you) can freely open issues, submit pull requests, and review pull requests.

Check out our dedicated [Bevy Organization document](/docs/the_bevy_organization.md) to learn more about how we're organized.

### Classifying PRs

Our merge strategy relies on the classification of PRs on two axes:

* How controversial are the design decisions
* How complex is the implementation

PRs with non-trivial design decisions are given the [`S-Controversial`] label. This indicates that
the PR needs more thorough design review or an [RFC](https://github.com/bevyengine/rfcs), if complex enough.

PRs that are non-trivial to review are given the [`D-Complex`] label. This indicates that the PR
should be reviewed more thoroughly and by people with experience in the area that the PR touches.

When making PRs, try to split out more controversial changes from less controversial ones, in order to make your work easier to review and merge.
It is also a good idea to try and split out simple changes from more complex changes if it is not helpful for then to be reviewed together.

Some things that are reason to apply the [`S-Controversial`] label to a PR:

1. Changes to a project-wide workflow or style.
2. New architecture for a large feature.
3. Serious tradeoffs were made.
4. Heavy user impact.
5. New ways for users to make mistakes (footguns).
6. Adding a dependency
7. Touching licensing information (due to level of precision required).
8. Adding root-level files (due to the high level of visibility)

Some things that are reason to apply the [`D-Complex`] label to a PR:

1. Introduction or modification of soundness relevant code (for example `unsafe` code)
2. High levels of technical complexity.
3. Large-scale code reorganization

Examples of PRs that are not [`S-Controversial`] or [`D-Complex`]:

* Fixing dead links.
* Removing dead code or unused dependencies.
* Typo and grammar fixes.
* [Add `Mut::reborrow`](https://github.com/bevyengine/bevy/pull/7114)
* [Add `Res::clone`](https://github.com/bevyengine/bevy/pull/4109)

Examples of PRs that are [`S-Controversial`] but not [`D-Complex`]:

* [Implement and require `#[derive(Component)]` on all component structs](https://github.com/bevyengine/bevy/pull/2254)
* [Use default serde impls for Entity](https://github.com/bevyengine/bevy/pull/6194)

Examples of PRs that are not [`S-Controversial`] but are [`D-Complex`]:

* [Ensure `Ptr`/`PtrMut`/`OwningPtr` are aligned in debug builds](https://github.com/bevyengine/bevy/pull/7117)
* [Replace `BlobVec`'s `swap_scratch` with a `swap_nonoverlapping`](https://github.com/bevyengine/bevy/pull/4853)

Examples of PRs that are both [`S-Controversial`] and [`D-Complex`]:

* [bevy_reflect: Binary formats](https://github.com/bevyengine/bevy/pull/6140)

Some useful pull request queries:

* [PRs which need reviews and are not `D-Complex`](https://github.com/bevyengine/bevy/pulls?q=is%3Apr+-label%3AD-Complex+-label%3AS-Ready-For-Final-Review+-label%3AS-Blocked++)
* [`D-Complex` PRs which need reviews](https://github.com/bevyengine/bevy/pulls?q=is%3Apr+label%3AD-Complex+-label%3AS-Ready-For-Final-Review+-label%3AS-Blocked)

[`S-Controversial`]: https://github.com/bevyengine/bevy/pulls?q=is%3Aopen+is%3Apr+label%3AS-Controversial
[`D-Complex`]: https://github.com/bevyengine/bevy/pulls?q=is%3Aopen+is%3Apr+label%3AD-Complex

### Prioritizing PRs and issues

We use [Milestones](https://github.com/bevyengine/bevy/milestones) to track issues and PRs that:

* Need to be merged/fixed before the next release. This is generally for extremely bad bugs i.e. UB or important functionality being broken.
* Would have higher user impact and are almost ready to be merged/fixed.

There are also two priority labels: [`P-Critical`](https://github.com/bevyengine/bevy/issues?q=is%3Aopen+is%3Aissue+label%3AP-Critical) and [`P-High`](https://github.com/bevyengine/bevy/issues?q=is%3Aopen+is%3Aissue+label%3AP-High) that can be used to find issues and PRs that need to be resolved urgently.

## Making changes to Bevy

Most changes don't require much "process". If your change is relatively straightforward, just do the following:

1. A community member (that's you!) creates one of the following:
    * [GitHub Discussions]: An informal discussion with the community. This is the place to start if you want to propose a feature or specific implementation.
    * [Issue](https://github.com/bevyengine/bevy/issues): A formal way for us to track a bug or feature. Please look for duplicates before opening a new issue and consider starting with a Discussion.
    * [Pull Request](https://github.com/bevyengine/bevy/pulls) (or PR for short): A request to merge code changes. This starts our "review process". You are welcome to start with a pull request, but consider starting with an Issue or Discussion for larger changes (or if you aren't certain about a design). We don't want anyone to waste their time on code that didn't have a chance to be merged! But conversely, sometimes PRs are the most efficient way to propose a change. Just use your own judgement here.
2. Other community members review and comment in an ad-hoc fashion. Active subject matter experts may be pulled into a thread using `@mentions`. If your PR has been quiet for a while and is ready for review, feel free to leave a message to "bump" the thread, or bring it up on [Discord](https://discord.gg/bevy) in an appropriate engine development channel.
3. Once they're content with the pull request (design, code quality, documentation, tests), individual reviewers leave "Approved" reviews.
4. After consensus has been reached (typically two approvals from the community or one for extremely simple changes) and CI passes, the [S-Ready-For-Final-Review](https://github.com/bevyengine/bevy/issues?q=is%3Aopen+is%3Aissue+label%3AS-Ready-For-Final-Review) label is added.
5. When they find time, someone with merge rights performs a final code review and merges the PR using [Bors](https://bors.tech/) by typing `bors r+`.

### Complex changes

Individual contributors often lead major new features and reworks. However these changes require more design work and scrutiny. Complex changes like this tend to go through the following lifecycle:

1. A need or opportunity is identified and an issue is made, laying out the general problem.
2. As needed, this is discussed further on that issue thread, in cross-linked [GitHub Discussion] threads, or on [Discord] in the Engine Development channels.
3. Either a Draft Pull Request or an RFC is made. As discussed in the [RFC repo](https://github.com/bevyengine/rfcs), complex features need RFCs, but these can be submitted before or after prototyping work has been started.
4. The community as a whole helps improve the Draft PR and/or RFC, leaving comments, making suggestions, and submitting pull requests to the original branch.
5. Once the RFC is merged and/or the Draft Pull Request is transitioned out of draft mode, the [normal change process outlined in the previous section](#making-changes-to-bevy) can begin.

## How you can help

If you've made it to this page, you're probably already convinced that Bevy is a project you'd like to see thrive.
But how can *you* help?

No matter your experience level with Bevy or Rust or your level of commitment, there are ways to meaningfully contribute.
Take a look at the sections that follow to pick a route (or five) that appeal to you.

If you ever find yourself at a loss for what to do, or in need of mentorship or advice on how to contribute to Bevy, feel free to ask in [Discord] and one of our more experienced community members will be happy to help.

### Battle-testing Bevy

Ultimately, Bevy is a tool that's designed to help people make cool games.
By using Bevy, you can help us catch bugs, prioritize new features, polish off the rough edges, and promote the project.

If you need help, don't hesitate to ask for help on [GitHub Discussions], [Discord], or [reddit](https://www.reddit.com/r/bevy). Generally you should prefer asking questions as [GitHub Discussions] as they are more searchable.

When you think you've found a bug, missing documentation, or a feature that would help you make better games, please [file an issue](https://github.com/bevyengine/bevy/issues/new/choose) on the main `bevy` repo.

Do your best to search for duplicate issues, but if you're unsure, open a new issue and link to other related issues on the thread you make.

Once you've made something that you're proud of, feel free to drop a link, video, or screenshot in `#showcase` on [Discord]!
If you release a game on [itch.io](https://itch.io/games/tag-bevy) we'd be thrilled if you tagged it with `bevy`.

### Teaching others

Bevy is still very young, and light on documentation, tutorials, and accumulated expertise.
By helping others with their issues, and teaching them about Bevy, you will naturally learn the engine and codebase in greater depth while also making our community better!

Some of the best ways to do this are:

* Answering questions on [GitHub Discussions], [Discord], and [reddit](https://www.reddit.com/r/bevy).
* Writing tutorials, guides, and other informal documentation and sharing them on [Bevy Assets](https://github.com/bevyengine/bevy-assets).
* Streaming, writing blog posts about creating your game, and creating videos. Share these in the `#devlogs` channel on [Discord]!

### Writing plugins

You can improve Bevy's ecosystem by building your own Bevy Plugins and crates.

Non-trivial, reusable functionality that works well with itself is a good candidate for a plugin.
If it's closer to a snippet or design pattern, you may want to share it with the community on [Discord], Reddit, or [GitHub Discussions] instead.

Check out our [plugin guidelines](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md) for helpful tips and patterns.

### Fixing bugs

Bugs in Bevy (or the associated website / book) are filed on the issue tracker using the [`C-Bug`](https://github.com/bevyengine/bevy/issues?q=is%3Aissue+is%3Aopen+label%3AC-Bug) label.

If you're looking for an easy place to start, take a look at the [`D-Good-First-Issue`](https://github.com/bevyengine/bevy/issues?q=is%3Aopen+is%3Aissue+label%3AD-Good-First-Issue) label, and feel free to ask questions on that issue's thread in question or on [Discord].
You don't need anyone's permission to try fixing a bug or adding a simple feature, but stating that you'd like to tackle an issue can be helpful to avoid duplicated work.

When you make a pull request that fixes an issue, include a line that says `Fixes #X` (or "Closes"), where `X` is the issue number.
This will cause the issue in question to be closed when your PR is merged.

General improvements to code quality are also welcome!
Bevy can always be safer, better tested, and more idiomatic.

### Writing docs

Like every other large, rapidly developing open source library you've ever used, Bevy's documentation can always use improvement.
This is incredibly valuable, easily distributed work, but requires a bit of guidance:

* Inaccurate documentation is worse than no documentation: prioritize fixing broken docs.
* Bevy is remarkably unstable: before tackling a new major documentation project, check in with the community on Discord or GitHub (making an issue about specific missing docs is a great way to plan) about the stability of that feature and upcoming plans to save yourself heartache.
* Code documentation (doc examples and in the examples folder) is easier to maintain because the compiler will tell us when it breaks.
* Inline documentation should be technical and to the point. Link relevant examples or other explanations if broader context is useful.
* The Bevy book is hosted on the `bevy-website` repo and targeted towards beginners who are just getting to know Bevy (and perhaps Rust!).
* Accepted RFCs are not documentation: they serve only as a record of accepted decisions.

[docs.rs](https://docs.rs/bevy) is built from out of the last release's documentation, which is written right in-line directly above the code it documents.
To view the current docs on `main` before you contribute, clone the `bevy` repo, and run `cargo doc --open` or go to [dev-docs.bevyengine.org](https://dev-docs.bevyengine.org/),
which has the latest API reference built from the repo on every commit made to the `main` branch.

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

There are three main places you can check for things to review:

1. Pull request which are ready and in need of more reviews on [bevy](https://github.com/bevyengine/bevy/pulls?q=is%3Aopen+is%3Apr+-label%3AS-Ready-For-Final-Review+-draft%3A%3Atrue+-label%3AS-Needs-RFC+-reviewed-by%3A%40me+-author%3A%40me)
2. Pull requests on [bevy](https://github.com/bevyengine/bevy/pulls) and the [bevy-website](https://github.com/bevyengine/bevy-website/pulls) repos.
3. [RFCs](https://github.com/bevyengine/rfcs), which need extensive thoughtful community input on their design.

Official focus areas and work done by @cart go through this review process as well.
Not even our project lead is exempt from reviews and RFCs!
By giving feedback on this work (and related supporting work), you can help us make sure our releases are both high-quality and timely.

Finally, if nothing brings you more satisfaction than seeing every last issue labeled and all resolved issues closed, feel free to message @cart for a Bevy org role to help us keep things tidy.
As discussed in our [*Bevy Organization doc*](/docs/the_bevy_organization.md), this role only requires good faith and a basic understanding of our development process.

### How to adopt pull requests

Occasionally authors of pull requests get busy or become unresponsive, or project members fail to reply in a timely manner.
This is a natural part of any open source project.
To avoid blocking these efforts, these pull requests may be *adopted*, where another contributor creates a new pull request with the same content.
If there is an old pull request that is without updates, comment to the organization whether it is appropriate to add the
*[S-Adopt-Me](https://github.com/bevyengine/bevy/labels/S-Adopt-Me)* label, to indicate that it can be *adopted*.
If you plan on adopting a PR yourself, you can also leave a comment on the PR asking the author if they plan on returning.
If the author gives permission or simply doesn't respond after a few days, then it can be adopted.
This may sometimes even skip the labeling process since at that point the PR has been adopted by you.

With this label added, it's best practice to fork the original author's branch.
This ensures that they still get credit for working on it and that the commit history is retained.
When the new pull request is ready, it should reference the original PR in the description.
Then notify org members to close the original.

* For example, you can reference the original PR by adding the following to your PR description:

`Adopted #number-original-pull-request`

### Contributing code

Bevy is actively open to code contributions from community members.
If you're new to Bevy, here's the workflow we use:

1. Fork the `bevyengine/bevy` repository on GitHub. You'll need to create a GitHub account if you don't have one already.
2. Make your changes in a local clone of your fork, typically in its own new branch.
   1. Try to split your work into separate commits, each with a distinct purpose. Be particularly mindful of this when responding to reviews so it's easy to see what's changed.
   2. Tip: [You can set up a global `.gitignore` file](https://docs.github.com/en/get-started/getting-started-with-git/ignoring-files#configuring-ignored-files-for-all-repositories-on-your-computer) to exclude your operating system/text editor's special/temporary files. (e.g. `.DS_Store`, `thumbs.db`, `*~`, `*.swp` or `*.swo`) This allows us to keep the `.gitignore` file in the repo uncluttered.
3. To test CI validations locally, run the `cargo run -p ci` command. This will run most checks that happen in CI, but can take some time. You can also run sub-commands to iterate faster depending on what you're contributing:
    * `cargo run -p ci -- lints` - to run formatting and clippy
    * `cargo run -p ci -- test` - to run tests
    * `cargo run -p ci -- doc` - to run doc tests and doc checks
    * `cargo run -p ci -- compile` - to check that everything that must compile still does (examples and benches), and that some that shouldn't still don't ([`crates/bevy_ecs_compile_fail_tests`](./crates/bevy_ecs_compile_fail_tests))
    * to get more information on commands available and what is run, check the [tools/ci crate](./tools/ci)

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

* Try to loosely follow the workflow in [*Making changes to Bevy*](#making-changes-to-bevy).
* Consult the [style guide](.github/contributing/engine_style_guide.md) to help keep our code base tidy.
* Explain what you're doing and why.
* Document new code with doc comments.
* Include clear, simple tests.
* Add or improve the examples when adding new user-facing functionality.
* Break work into digestible chunks.
* Ask for any help that you need!

Your first PR will be merged in no time!

No matter how you're helping: thanks for contributing to Bevy!

[GitHub Discussions]: https://github.com/bevyengine/bevy/discussions "GitHub Discussions"
[Discord]: https://discord.gg/bevy "Discord"
