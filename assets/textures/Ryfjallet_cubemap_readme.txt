Modifications
=============

The original work, as attributed below, has been modified as follows using the ImageMagick tool:

mogrify -resize 256x256 -format png *.jpg
convert posx.png negx.png posy.png negy.png posz.png negz.png -gravity center -append cubemap.png

Author
======

This is the work of Emil Persson, aka Humus.
http://www.humus.name



License
=======

This work is licensed under a Creative Commons Attribution 3.0 Unported License.
http://creativecommons.org/licenses/by/3.0/
