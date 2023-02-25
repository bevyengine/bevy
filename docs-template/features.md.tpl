# Cargo Features

## Default Features

|feature name|description|
|-|-|
{% for feature in features %}{% if feature.is_default %}|{{ feature.name }}|{{ feature.description }}|
{% endif %}{% endfor %}

## Optional Features

|feature name|description|
|-|-|
{% for feature in features %}{% if feature.is_default == False %}|{{ feature.name }}|{{ feature.description }}|
{% endif %}{% endfor %}
