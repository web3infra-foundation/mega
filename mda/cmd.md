# CMD

Execute the following command in the 'mega' directory

## Generate mda files

Generate mda for one single file

```shell
cargo run -p mda -- --action=generate --train=mda/test/train/1.jpg --anno=mda/test/anno/1.txt --output=mda/test/output/ --tags=image,dog
```

Generate mda for files in a folder

```shell
cargo run -p mda -- --action=generate --train=mda/test/train/ --anno=mda/test/anno/ --output=mda/test/output/ --tags=image,dog
```



## List info for mda files

List info for one single file

```shell
cargo run -p mda -- --action=list --mda=mda/test/output/1.mda
```

List info for files in a folder

```shell
cargo run -p mda -- --action=list --mda=mda/test/output/
```



## Extract files from mda

Extract training data and annotation data from one single file

```shell
cargo run -p mda -- --action=extract --mda=mda/test/output/1.mda --train=mda/test/output/ --anno=mda/test/output/

```

Extract training data and annotation data from .mda files 

```shell
cargo run -p mda -- --action=extract --mda=mda/test/output/ --train=mda/test/output/ --anno=mda/test/output/

```



## Update annotation data

```
cargo run -p mda -- --action=extract --mda=mda/test/output/1.mda --train=mda/test/output/ --anno=mda/test/output/ --rev=0
```

rev: version

## List versions

list versions of the targeted file

```
cargo run -p mda -- --action=version --mda=mda/test/output/1.mda
```

