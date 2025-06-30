if [ -n "$(git status --porcelain)" ]; then
    echo "You have local changes!"
    exit 1
fi

pushd crates

for crate in `cargo package --workspace 2>&1 | grep Packaging | sed 's_.*crates/\(.*\))_\1_' | grep -v Packaging`
do
  echo "Publishing ${crate}"
  pushd "$crate"
  cargo publish
  popd
done

popd

echo "Publishing root crate"
cargo publish
