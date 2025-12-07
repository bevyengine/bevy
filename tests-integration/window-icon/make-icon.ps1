# create .ico
magick convert -background transparent "../../assets/branding/icon.png" -define icon:auto-resize=16,24,32,48,64,72,96,128,256 "favicon.ico"