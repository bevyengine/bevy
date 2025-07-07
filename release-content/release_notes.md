# Bevy's Release Notes Process

Hi! Did someone add `M-Needs-Release-Note` to your PR? If so, you're in the right place.
Let's talk about how this process works.

When we make high-impact changes to Bevy, we need to communicate them to users (and potential users!).
For the most part, this is done via our [famously in-depth release notes](https://bevy.org/news/).
To avoid a crunch at the end of the cycle as we *write* all of these,
Bevy asks authors (and reviewers) to write draft release notes as part of the pull requests to add high-impact features.

## Where to put your release notes

Each major Bevy version (e.g. 0.12, or 2.0) will get its own set of release notes.
The draft release notes are organized in the `bevyengine/bevy/release-content/release-notes` folder.

When we publish our first release candidate for a cycle, these notes are merged together and moved from `bevyengine/bevy` into `bevyengine/bevy-website`,
where they will receive a final editing pass and any multimedia.

To start a new release note, copy-paste [the template](./release_notes_template.md) into a new file in the `release-notes` folder.

## What to put in your draft release notes

Release notes are intended to capture the essence of:

- what has been changed or added?
- why is this a big deal for users?
- how can they use it?

Draft release notes *do not need to be polished*: it's okay if you're not a native English speaker or aren't a wordsmith.
Editing is easy: as someone with the expertise needed to implement an awesome feature we want a rough expert's perspective that we can shape into something that reads smoothly and has a consistent voice.

Images and videos are lovely: shiny screenshots of rendering features, diagrams, performance metrics, and cool examples are all a great choice.
However, **do not put multimedia content in this folder**.
We want to avoid bloating the git repo for `bevyengine/bevy`, which can cause problems for contributors (and GitHub).
Instead, drop them in your PR description and we'll collect them as we're finalizing the release notes.

## Grouping content into release notes

Release notes should be organized by "rough feature", not "per PR".
Bevy users don't care if the work was done in 17 PRs, or a single 10k line PR.

As a result, each draft release note should be given a clear name matching the section title,
and related PRs (and their authors!) should be collected into the metadata listed in those markdown files.

If you make changes or extensions to an upcoming major feature, you should probably revise the release note for that feature.
