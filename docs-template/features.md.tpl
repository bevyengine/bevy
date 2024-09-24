<!-- MD041 - This file will be included in docs and should not start with a top header -->
<!-- markdownlint-disable-file MD041 -->

## Cargo Features

Bevy exposes many features to customise the engine. Enabling them add functionalities but often come at the cost of longer compilation times and extra dependencies.

### Default Features

The default feature set enables most of the expected features of a game engine, like rendering in both 2D and 3D, asset loading, audio and UI. To help reduce compilation time, consider disabling default features and enabling only those you need.

|feature name|description|
|-|-|
{% for feature in features %}{% if feature.is_default %}|{{ feature.name }}|{{ feature.description }}|
{% endif %}{% endfor %}
### Optional Features

|feature name|description|
|-|-|
{% for feature in features %}{% if feature.is_default == False %}|{{ feature.name }}|{{ feature.description }}|
{% endif %}{% endfor %}