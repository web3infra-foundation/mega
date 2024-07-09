mkdir /tmp/lower
mkdir /tmp/upper
mkdir /tmp/lower/a
mkdir /tmp/lower/b
mkdir /tmp/lower/c
mkdir /tmp/test_mount
cd /tmp/lower/a && echo "Hello" > a.txt


cargo test --package scorpio --lib -- overlayfs::tests::test_overlayfs --exact --show-output