# Magic Leap demo

First, install v0.20.0 or later of the Magic Leap SDK. By default this is installed in `MagicLeap/mlsdk/<version>`, for example:
```
  export MAGICLEAP_SDK=~/MagicLeap/mlsdk/v0.20.0
```
  You will also need a signing certificate.
```
  export MLCERT=~/MagicLeap/cert/mycert.cert
```

Now build the pathfinder demo library and `.mpk` archive:
```
  cd demo/pathfinder
  make release
```

The `.mpk` can be installed:
```
  make install
```
and run:
```
  make run
```
