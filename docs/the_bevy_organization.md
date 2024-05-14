# The Bevy Organization

The Bevy Organization is the group of people responsible for stewarding the Bevy project. It handles things like merging pull requests, choosing project direction, managing bugs / issues / feature requests, running the Bevy website, controlling access to secrets, defining and enforcing best practices, etc.

Note that you _do not_ need to be a member of the Bevy Organization to contribute to Bevy. Community contributors (this means you) can freely open issues, submit pull requests, and review pull requests.

The Bevy Organization is currently broken up into the following roles:

## Project Lead

Project Leads have the final call on all design and code changes within Bevy. This is to ensure a coherent vision and consistent quality of code. They are responsible for representing the project publicly and interacting with other entities (companies, organizations, etc) on behalf of the project. They choose how the project is organized, which includes how responsibility is delegated. Project Leads implicitly have the power of other roles (Maintainer, Subject Matter Expert, etc).

@cart is, for now, our singular project lead. @cart tries to be accountable: open to new ideas and to changing his mind in the face of compelling arguments or community consensus.

## Maintainer

Maintainers have merge rights in Bevy repos. They assess the scope of pull requests and whether they fit into the Bevy project's vision. They also serve as representatives of the Bevy project and are often the interface between the Bevy community and the Bevy project. They assist the Project Leads in moderating the community, handling administrative tasks, defining best practices, choosing project direction, and deciding how the project is organized.

Maintainers abide by the following rules when merging pull requests:

1. Trivial PRs can be merged without approvals.
2. Relatively uncontroversial PRs can be merged following approval from at least two community members (including Maintainers) with appropriate expertise.
3. Controversial PRs cannot be merged unless they have the approval of a Project Lead or two Subject Matter Experts (in the "area" of the PR).
4. If two Maintainers have approved a controversial PR they can "start the clock" on a PR by adding it to [this queue](https://github.com/orgs/bevyengine/projects/6). If 45 days elapse without SME or Project Lead action (approval, feedback or an explicit request to defer), the PR can be merged by maintainers.

We choose new Maintainers carefully and only after they have proven themselves in the Bevy community. Maintainers must have a proven track record of the following:

1. **A strong understanding of the Bevy project as a whole**: our vision, our development process, and our community
2. **Solid technical skills and code contributions across most engine areas**: Maintainers must be able to evaluate the scope of pull requests, provide complete code reviews, ensure the appropriate people have signed off on a PR, and decide if changes align with our vision for Bevy. This can only be done if Maintainers are technical experts, both generically across engine subject areas, and more specifically in the Bevy codebase.
3. **Great social skills**: Maintainers regularly deal with and resolve "community issues". They must always present a professional and friendly face. They are representatives of the project and their actions directly reflect our goals and values. Working with them should not be painful.
4. **Thorough reviews of other peoples' PRs**: Maintainers are the last line of defense when protecting project vision and code quality. They are also often the first people new contributors interact with. They must have a history of leaving thorough and helpful code reviews.
5. **Ethical and trustworthy behavior**: Maintainers are granted significant administrative permissions. They must be trustable.

To make it easy to reach consensus, hold a high quality bar, and synchronize vision, we intentionally keep the Maintainer team small. We choose new maintainers carefully and only after they have proven themselves in the Bevy community.

If you are interested in a Maintainer role and believe you meet these criteria, reach out to one of our Project Leads or Maintainers. One month after every Bevy release Maintainers and Project Leads will evaluate the need for new roles, review candidates, and vote. Bringing in a new Maintainer requires unanimous support from all Project Leads and Maintainers.

Check out the [Bevy People](https://bevyengine.org/community/people/#the-bevy-organization) page for the current list of maintainers.

## Subject Matter Expert (SME)

Subject Matter Experts are members of the Bevy Organization that have proven themselves to be experts in a given development area (Rendering, Assets, ECS, UI, etc) and have a solid understanding of the Bevy Organization's vision for that area. They are great people to reach out to if you have questions about a given area of Bevy.

SME approvals count as "votes" on controversial PRs (provided the PR is in their "subject area"). This includes [RFCs](https://github.com/bevyengine/rfcs). If a controversial PR has two votes from Subject Matter Experts in that PR's area, it can be merged without Project Lead approval. If a SME creates a PR in their subject area, this does count as a vote. However, Project Leads have the right to revert changes merged this way, so it is each SME's responsibility to ensure they have synced up with the Project Lead's vision. Additionally, when approving a design, consensus between SMEs and Project Leads (and ideally most of the wider Bevy community) is heavily encouraged. Merging without consensus risks fractured project vision and/or ping-ponging between designs. The "larger" the impact of a design, the more critical it is to establish consensus.

We choose new SMEs carefully and only after they have proven themselves in the Bevy community. SMEs must have a proven track record of the following:

1. **Designing and contributing to foundational pieces in their subject area**: SMEs are responsible for building and extending the foundations of a given subject area. They must have a history of doing this before becoming an SME.
2. **Thorough reviews of other peoples' PRs in their subject area**: Within a subject area, SMEs are responsible for guiding people in the correct technical direction and only approving things aligned with that vision. They must have a history of doing this before becoming an SME.
3. **Great social skills**: Within a subject area, SMEs are responsible for reviewing peoples' code, communicating project vision, and establishing consensus. They are representatives of the project and their actions directly reflect our goals and values. Working with them should not be painful.

To make it easy to reach consensus, hold a high quality bar, and synchronize vision, we intentionally keep the number of SMEs in a given area small: 2 is the absolute minimum (to allow voting to occur), 3 is preferred, and 4 will be allowed in some cases. Bevy Organization members can be SMEs in more than one area, and Maintainers can also be SMEs.

If you are interested in a SME role and believe you meet these criteria, reach out to one of our Project Leads or Maintainers. One month after every Bevy release Maintainers and Project Leads will evaluate the need for new roles, review candidates, and vote. Bringing in a new SME requires the support of the Project Leads and half of the Maintainers (however unanimous support is preferred).

Check out the [Bevy People](https://bevyengine.org/community/people/#the-bevy-organization) page for the current list of SMEs.

## Bevy Org Member / Triage Team

[Bevy Org members](https://github.com/orgs/bevyengine/people) are contributors who:

1. Have actively engaged with Bevy development.
2. Have demonstrated themselves to be polite and welcoming representatives of the project with an understanding of our goals and direction.
3. Have asked to join the Bevy Org. Reach out to @cart on [Discord](https://discord.gg/bevy) or email us at <bevyengine@gmail.com> if you are interested. Everyone is welcome to do this. We generally accept membership requests, so don't hesitate if you are interested!

All Bevy Org members are also Triage Team members. The Triage Team can label and close issues and PRs but do not have merge rights or any special authority within the community.

## Role Rotation

All Bevy Organization roles (excluding the Triage Team) have the potential for "role rotation". Roles like Project Lead, Maintainer, and SME are intentionally kept in limited supply to ensure a cohesive project vision. However these roles can be taxing, and qualified motivated people deserve a chance to lead. To resolve these issues, we plan on building in "role rotation". What this looks like hasn't yet been determined (as this issue hasn't come up yet and we are still in the process of scaling out our team), but we will try to appropriately balance the needs and desires of both current and future leaders, while also ensuring consistent vision and continuity for Bevy.

Additionally, if you are currently holding a role that you can no longer "meaningfully engage with", please reach out to the Project Leads and Maintainers about rotating out. We intentionally keep leadership roles in short supply to make it easier to establish consensus and encourage a cohesive project vision. If you hold a role but don't engage with it, you are preventing other qualified people from driving the project forward. Note that leaving a role doesn't need to be permanent. If you need to rotate out because your life is currently busy with work / life / school / etc, but later you find more time, we can discuss rotating back in!
