---
title: "Contextual theming"
pull_requests: [24969]
---

The Feathers `ThemeProps` structure has significantly changed; if you have created a custom theme,
you will need to reorganize it based on semantic tokens.

For a quick and easy port to start off from, you can create SemanticToken’s for every ThemeToken
you have used in your custom theme. Then, create a map from these ThemeTokens to your
SemanticTokens, and map these SemanticTokens to your desired colors.

From there, you can start combining any identical colors used to the same SemanticToken where
it makes sense in your application.
