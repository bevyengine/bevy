# Add scale factor to `Val::resolve` 

prs = [18164]

`Val::resolve` now has a scale factor parameter. To resolve a `Val` to a logical value pass in a `scale_factor` of `1.`. 