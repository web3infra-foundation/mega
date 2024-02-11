# MDA 0.1.0 - Data, Annotations, Versions, Together.

## 1 Introduction 

Managing data during large-scale training can be challenging due to the a high volume of files, fragmented data, and diverse annotations. The lack of a connection between training data and its annotation data creates difficulties in data management. Additionally, not being able to keep track of data changes makes it harder to manage data effectively.

To address these issues, we propose MDA, a file format that **integrates training data and their corresponding annotations**. And it also supports **version control for annotation data** during training. 

## 2 Features

- Integration of training data and annotations into a single file format.
- Support version control of annotations during training.
- Combination of training data, annotation data, and version information into a single file, improving data management efficiency.
- Addressing the separation between training data and annotations, enhancing research efficiency and data quality.
- Efficient tracking and management of different versions of annotations.

## 3 Technology Details

### 3.1 File Format Design

The MDA is implemented in Rust. MDA is a binary file that contains the training data and all versions of its all annotation data. MDA uses the "**.mda**" file extension, and the filename of an MDA file will match the filename of the extracted training data.

The file format design of MDA is shown in the figure.

![img](https://github.com/open-rust-initiative/mda/blob/main/assets/r6VFnpW-I9_bZ__p6IQyC4wR14fyAZ7vVFKHl6ItfM23ccst9qJESJUBCJkawOzVRrZM0kwG7AWgMjVg6yk2TVLDdwxsSH2EwreTmq6ekh8P4b9ROhNBeouxF0c7Ym3IbTtmjaFVe_FZ72ZikqZGaA.png?raw=true)

**MDA consists of four components: MDAIndex, MDAHeader, Training Data, and Annotation Data:**

1. **MDAIndex**: The index module records the data offset of each section in the binary file. This design aims to locate and access specific modules quickly.

![image-20230911135403622](https://github.com/open-rust-initiative/mda/blob/main/assets/image-20230911135403622.png?raw=true)

annotations_offset records multiple sets of AnnoOffset, capturing the data offsets of different annotation data.

![image-20230911135434488](https://github.com/open-rust-initiative/mda/blob/main/assets/image-20230911135434488.png?raw=true)

The following figure presents the specific design of MDAIndex.

![img](https://github.com/open-rust-initiative/mda/blob/main/assets/myY1LeW0whNCKj8eGVF-Wdq-03_HJNzahk761D0jk5DiTonrG1hUy2vFvqx91v-k3PSSiVrKRDLIu0UCQEQ44zkMxdq_g-U24aF4q2Eb7o9PAMZ4IXISxAbv1TdYMDhw6ntNqwJWnCRtrigUukTF9w.png?raw=true)

2. **MDAHeader**: The module records index information, labels, training data types, metadata, and other content. 

![image-20230911135515676](https://github.com/open-rust-initiative/mda/blob/main/assets/image-20230911135509423.png?raw=true)

The following figure presents the specific design of MDAHeader.

![img](https://github.com/open-rust-initiative/mda/blob/main/assets/EucZJh-k90OZyjRP99zkyqjG9I6B8S8mNmPDG7QoDmVtBxOTe1gUuBB5AXO-wdLRE4TygzhWiJ5GcXr2sLTE5Us3l8pMOaIg7C3hchgJ4qmeDLezL15fMoJ6KbGQs2pk8o2CNv9wcB69h_qi2cNjwQ.png?raw=true)



3. **Training Data**: The train_data module is used to store training data. 

4. **anno_data**: It contains all the annotations of the training data. The anno_data module is used to store annotated data and perform version control. The details are in 3.2.

### 3.2 Data Version Control

#### 3.2.1 Purpose

Build a data structure RevAnno to do version control of annotated data. It stores the differential content of data and implements version tracking and data storage optimization through incremental storage and data snapshots. RevAnno is designed based on the principles of Mercurial Revlog, aiming to reduce storage space and improve read-write efficiency.

#### 3.2.2 Working Principles

The figure shows the working principles of RevAnno.![img](https://github.com/open-rust-initiative/mda/blob/main/assets/TGMrfL76RV05bzoh7g7WvdBgmS8jk3NkPK9aCo8spNeve-PTN2HgM1CFAMmjDXAEgtILC8mgrnLuBlLM_FJdRunVfnMNKKxtiRX05TkNBMFD71nKqpp0fDFmpU-N0njXc6I9KhaVjsc8zs72iAA9rA.png?raw=true)
**Initial Storage**: RevAnno saves the complete annotated data during the first storage.
**Incremental Storage**: Subsequent storages only store the parts that differ from the previous version. Assuming the current version is the m-th storage, in the m+1-th storage, only the parts that are different from the previous m storages are saved, and so on. 

**Snapshot**: Periodically or upon specific events, RevAnno creates a data snapshot. The data snapshot saves the complete state of all data at that particular time, making it easy to revert to specific versions of the data. 

**Data Retrieval**: To view the complete data of a specific version, RevAnno needs to trace back to the initial version and apply the differences between each version one by one until reaching the desired version's complete data state. The following are the steps to obtain the complete data of a specific version:Retrieve the data from the initial version (usually the complete data from the first storage or snapshot data).Sequentially retrieve the differential data between each version and apply the differences to the data from the initial version. For example, starting from the second storage, retrieve the differential data between the second storage and the first storage, then apply it to the data from the initial version, resulting in the complete data of the second storage. Then, retrieve the differential data between the third storage and the second storage and apply it to the complete data from the second storage, resulting in the complete data of the third storage.Finally, obtain the complete data of the target version (the third storage).

#### 3.2.3 RevAnno Version Control

![image-20230911134627546](https://github.com/open-rust-initiative/mda/blob/main/assets/image-20230911134627546.png?raw=true)

Input data at rev=0:

1. Split the data into multiple DataBlocks based on the segmentation rules.
2. Generate RevAnnoEntry to record all DataBlocks stored in this session, the index (the order in which the array blocks compose the data), and the sequence number (starting from 0) for this record.
3. Create RevAnnoHeader based on RevAnnoEntry to record information about RevAnnoEntry, including data length, offset, snapshot status, etc.



Input data at 0 < rev < m:

1. Split the data into multiple DataBlocks based on the segmentation rules.
2. Compare the new DataBlocks with the existing ones (rev=0, 1, 2, etc.), and store only the different DataBlocks.
3. Generate RevAnnoEntry and RevAnnoHeader.



Input data at rev=m with the snapshot setting: 

1. Store the newly added DataBlocks and all the previous DataBlocks.
2. Generate RevAnnoEntry and RevAnnoHeader.



Input data at rev=m+1:

1. Split the data into multiple DataBlocks based on the segmentation rules.
2. Compare the new DataBlocks with the existing ones (rev=m), and store only the different DataBlocks.
3. Generate RevAnnoEntry and RevAnnoHeader.

#### ![img](https://github.com/open-rust-initiative/mda/blob/main/assets/PDGaQYmOfDt10_MYaf_ROKPLqtDvX64ZvXljE7pe0fWssoklWu7Sf1WsHFjI9YcB5pwnWf7n2JK7fRjHZETP2H3pw3VXxXLLpUWQ3EjGIUbLxWiCwYnUxm9UFec-cRd3iQrosfvHOjFjZv-knXbs-w.png?raw=true)![img](https://github.com/open-rust-initiative/mda/blob/main/assets/PE5CZ0M6nSfe-06ECDJgHpfQNft0jakF9PmKZArHwR_otnG230xpskAZQU89rcdxPS5qoyWW1hAeOugbqqb3eintlujtZsl8RgMV_JH-yabqyeBccSzFB5ALGwbHOf24ctRBFmBajzIPhRfV4JusLQ.png?raw=true) **3.3.4 File Read and Write** 

**When writing to an MDA file**, the process involves first writing the rev_anno_entries to the MDA file, recording the offset of each entry. Then, the rev_anno_headers are updated and written to the MDA file. The MDA file's header is also updated to include the offsets of rev_anno_headers and rev_anno_entries. 

**When reading the file**, the procedure starts by using MDAIndex's rev_anno_headers to extract the header data of rev_anno. Next, the required headers are calculated, and using the offset information in the headers, the corresponding entries are located, and the data is merged and restored accordingly.
![img](https://github.com/open-rust-initiative/mda/blob/main/assets/IfqD4eW_8igWvHY8hq__lbkfHjNHt_ejIXkPoc-dpMINXMVgJkg8HT-PhkSK2wg3Sx7rlXH9wigZuB9TLTc5ank9QubB5WGu3XBUG3CHsxiYS64YaGQjS54j1xW_Cu7WcjqMbFZEYDwtuxqv0NOCbg.png?raw=true)

### 3.3 Mapping Training Data and Annotation Data

MDA uses the following two method to map training data and annotation file:

**Read Annotation Data Folder:** Match training data and annotation data from different folders by using the same names. For example the data in Table 2, MDA map 000001.jpg with 000001.txt, 000002.jpg with 000002.txt.

Training Data Folder

```
Mode                 LastWriteTime         Length Name
----                 -------------         ------ ----
-a----          2023/9/3     13:16            120 000001.txt
-a----          2023/9/3     13:16            120 000002.txt
-a----          2023/9/3     13:16            120 000003.txt
-a----          2023/9/3     13:16            120 000004.txt
-a----          2023/9/3     13:16            120 000005.txt
-a----          2023/9/3     13:16            120 000006.txt
-a----          2023/9/3     13:16            119 000007.txt
-a----          2023/9/3     13:16            119 000008.txt
-a----          2023/9/3     13:16            120 000009.txt
-a----          2023/9/3     13:16            120 000010.txt
```

Annotation Data Folder

```
Mode                 LastWriteTime         Length Name
----                 -------------         ------ ----
-a----         2015/9/28     16:49          58479 000001.jpg
-a----         2015/9/28     16:49          32197 000002.jpg
-a----         2015/9/28     16:49          13344 000003.jpg
-a----         2015/9/28     16:49         759834 000004.jpg
-a----         2015/9/28     16:49          86846 000005.jpg
-a----         2015/9/28     16:49          30244 000006.jpg
-a----         2015/9/28     16:49          27917 000007.jpg
-a----         2015/9/28     16:49          59712 000008.jpg
-a----         2015/9/28     16:49         493575 000009.jpg
-a----         2015/9/28     16:49          25127 000010.jpg 
```

**Read Annotation Data File:** Read annotation data and its content line by line from the file. Each line contains an annotation data and its corresponding content.  For example:

```
The following data is part of list_bbox_celeba.txt in CelebA
202599
image_id x_1 y_1 width height
000001.jpg    95  71 226 313
000002.jpg    72  94 221 306
000003.jpg   216  59  91 126
000004.jpg   622 257 564 781
000005.jpg   236 109 120 166
000006.jpg   146  67 182 252
000007.jpg    64  93 211 292
000008.jpg   212  89 218 302
000009.jpg   600 274 343 475
000010.jpg   113 110 211 292
000011.jpg   166  68 125 173
000012.jpg   102  31 104 144
000013.jpg    89 132 247 342
000014.jpg   110 122 234 324
000015.jpg    93  86 190 263
000016.jpg    39  89 283 392
000017.jpg    40  64  62  86
```

There might be one or multiple annotation data, so a TOML file has been established to read this type of annotation data.

Annotation data can be configured in the TOML file.

- id: The "id" serves as the identifier for this group of annotation data. If the user does not specify an "id," the program will extract the filename of the annotation file to use as the "id."
- path: Path to the annotation data.
- start: Starting line of the annotation data.
- end: Ending line of the annotation data.title = "mda config"

For example, mda_anno_config.toml

```toml
title = "mda config"

[[annotation]]
id = "identify"
path = "D:/Workplace/internship/project/test_mda/anno/identity_CelebA.txt"
start = 1
end=50000
[[annotation]]
path = "D:/Workplace/internship/project/test_mda/anno/list_attr_celeba.txt"
start = 3

[[annotation]]
path = "D:/Workplace/internship/project/test_mda/anno/list_bbox_celeba.txt"
start = 3

[[annotation]]
id = "landmarks"
path = "D:/Workplace/internship/project/test_mda/anno/list_landmarks_celeba.txt"
start=3
end=1000

```

## 4 User Guide

```shell
Usage: mega.exe mda [OPTIONS]

Options:
      --action <ACTION>            5 actions: generate, extract, list, group, version, update
      --train <TRAIN>              Training data file/folder path
      --anno <ANNO>                Annotation data file/folder path
      --annos <ANNOS>              Annotation data file/folder path, separated by commas
      --output <OUTPUT>            Output data file/folder path
      --mda <MDA>                  MDA data file/folder path
      --tags <TAGS>                Tags for MDA files
      --threads <THREADS>          Maximum number of threads [default: 10]
      --rev <REV>                  The version of MDA file [default: -1]
      --start <START>              Read from which line of the annotation file [default: 1]
      --end <END>                  Read from which line of the annotation file [default: 0]
      --format <FORMAT>            The type of the annotation data: txt,json [default: txt]
      --anno-config <ANNO_CONFIG>  Combined Annotation data config
      --group <GROUP>              The group of the annotation data [default: NONE]
      --mode <MODE>                The generation mode: one, multiple, combine
  -h, --help                       Print help (see more with '--help')
  -V, --version                    Print version
```

### 4.1 Generate MDA Files

Generating .mda file by specifying paths for training and annotation data.

Specify --action=generate, and then choose the mode based on the situation. There are three modes: one, multiple, and combine.

- In the 'one' mode, each training data corresponds to one annotation data when annotation data is in individual files.
- In the 'multiple' mode, each training data corresponds to multiple annotation data when annotation data is in individual files.
- In the 'combine' mode, all annotation data is present in a single file.
The following provides specific explanations for handling different scenarios.

1. In the 'one' mode, each training data corresponds to one annotation data when annotation data is in individual files.

	Generate the mda file for one training data, for example:

```shell
cargo run mda --action=generate --mode=one --train=tests/mda/data/train/1.jpg --anno=tests/mda/data/anno/anno1/1.txt --output=tests/mda/output/one/  --tags=dog
```

Generate mda files for multiple training data within one directory, for example:

```shell
cargo run mda --action=generate --mode=one --train=tests/mda/data/train/ --anno=tests/mda/data/anno/anno1/ --output=tests/mda/output/one/  --tags=cat,dog --threads=10
```

2. In the 'multiple' mode, each training data corresponds to multiple annotation data when annotation data is in individual files.
   Generate the mda file for one training data with multiple annotation data, for example:

```shell
cargo run mda --action=generate --mode=one --train=tests/mda/data/train/1.jpg --anno=tests/mda/data/anno/anno1/1.txt --output=tests/mda/output/one/  --tags=dog
```

​	Generate mda files for multiple training data with multiple annotation data within one directory, for example:

```shell
cargo run mda --action=generate --mode=multiple --train=tests/mda/data/train/ --annos=tests/mda/data/anno/anno1/,tests/mda/data/anno/anno2/,tests/mda/data/anno/anno3/ --output=tests/mda/output/multiple/
```

3. In the 'combine' mode, all annotation data is present in a single file.
   For example:

```shell
cargo run mda --action=generate --mode=combine  --train=tests/mda/celeba/train/ --anno-config=tests/mda/celeba/mda_config.toml --output=tests/mda/output/combine/  --tags=face  --threads=10
```

-–anno-config record the information of the annotation data, for example:

```toml
title = "mda config"

[[annotation]]
id = "identify"
path = "D:/Workplace/internship/project/test_mda/anno/identity_CelebA.txt"
start = 1
end=51

[[annotation]]
path = "D:/Workplace/internship/project/test_mda/anno/list_attr_celeba.txt"
start = 3
end=53

[[annotation]]
path = "D:/Workplace/internship/project/test_mda/anno/list_bbox_celeba.txt"
start = 3
end=53

[[annotation]]
id = "landmarks"
path = "D:/Workplace/internship/project/test_mda/anno/list_landmarks_celeba.txt"
start=3
end=53
```

### 4.2 List MDA File Information

**List Annotation Group**
List all types of annotation data. For example:

```shell
cargo run mda --action=group --mda=tests/mda/output/combine/000001.mda
```

```shell
Output: 
List the 4 different groups annotation data for this training data
+----+------------------+
| ID | Annotation Group |
+----+------------------+
| 1  | identify         |
+----+------------------+
| 2  | list_attr_celeba |
+----+------------------+
| 3  | list_bbox_celeba |
+----+------------------+
| 4  | landmarks        |
+----+------------------+
```

**List Tags and MetaData**
List data information from .mda file. Get the basic information of the training data and get the versions of the annotation data. For example:

```shell
cargo run mda --action=list --mda=tests/mda/output/combine/000001.mda
```

```shell
Output
+-------------------------------------+-------------------+----------------------+------+--------------------------------------------------------------------------+
| MDA File                            | MDA Header Offset | Training Data Offset | Tags | Training MetaData                                                        |
+-------------------------------------+-------------------+----------------------+------+--------------------------------------------------------------------------+
| tests/mda/output/combine/000001.mda | 169               | 282                  | face | ImageMetaData { size: (178, 218), channel_count: 3, color_space: "RGB" } |
+-------------------------------------+-------------------+----------------------+------+--------------------------------------------------------------------------+


```

```shell
cargo run mda --action=list --mda=tests/mda/output/combine/
```

```shell
Output
+-------------------------------------+-------------------+----------------------+------+--------------------------------------------------------------------------+
| MDA File                            | MDA Header Offset | Training Data Offset | Tags | Training MetaData                                                        |
+-------------------------------------+-------------------+----------------------+------+--------------------------------------------------------------------------+
| tests/mda/output/combine/000001.mda | 169               | 282                  | face | ImageMetaData { size: (178, 218), channel_count: 3, color_space: "RGB" } |
+-------------------------------------+-------------------+----------------------+------+--------------------------------------------------------------------------+
| tests/mda/output/combine/000002.mda | 169               | 282                  | face | ImageMetaData { size: (178, 218), channel_count: 3, color_space: "RGB" } |
+-------------------------------------+-------------------+----------------------+------+--------------------------------------------------------------------------+
| tests/mda/output/combine/000003.mda | 169               | 282                  | face | ImageMetaData { size: (178, 218), channel_count: 3, color_space: "RGB" } |
+-------------------------------------+-------------------+----------------------+------+--------------------------------------------------------------------------+
…
```

**List Versions**
List all versions of the targeted annotation data.  

```shell
cargo run mda --action=version --mda=tests/mda/output/combine/000001.mda --group="landmarks"
```

```shell
Output
Data Version for "tests/mda/output/one/1.mda", anno group: "anno1"
+-----+--------+--------+
| rev | offset | length |
+-----+--------+--------+
| 0   | 131932 | 114    |
+-----+--------+--------+
| 1   | 132046 | 132    |
+-----+--------+--------+
| 2   | 132178 | 163    |
+-----+--------+--------+
| 3   | 132341 | 94     |
+-----+--------+--------+
```

### 4.3 Update MDA Files

Update .mda file. To update the mda file, specify the modified training data path, annotation data path, and the path of the previous version of the .mda file.

Update one mda file, for example:

```shell
cargo run mda --action=update --mda=tests/mda/output/one/1.mda  --anno=tests/mda/data/anno/anno1/1.txt --group="anno1"
```

Update some mda files in a directory, for example:

```shell
cargo run mda --action=update --mda=tests/mda/output/one/  --anno=tests/mda/data/anno/anno1/ --group="anno1"
```

Update some annotation data in annotation combined case, for example:

```shell
cargo run mda --action=update --mda=tests/mda/output/one/  --anno=tests/mda/data/anno/anno1/ --group="anno1"
```



### 4.4 Extract MDA Files

Extract data from .mda file. The training data and annotation data can be extracted from the MDA file. The previous version of annotation data can also be extracted. 

Extract training data and annotation data. If users do not assign the version, it will extract the latest version of the data. For example:

```shell
cargo run mda --action=extract --mda=tests/mda/output/combine/000001.mda --train=tests/mda/extract/train/ --anno=tests/mda/extract/anno/  --group=list_attr_celeba 
```

Extract training data and annotation data from a targeted version. For example:

```shell
cargo run mda --action=extract --mda=tests/mda/output/combine/000001.mda --train=tests/mda/extract/train/ --anno=tests/mda/extract/anno/  --group=list_attr_celeba --rev=2
```

## 5 Applying MDA to the CelebA

In this section, we evaluate the functionality and performance of MDA tools a large-scale dataset, [CelebA](https://mmlab.ie.cuhk.edu.hk/projects/CelebA.html). For this evaluation, we have selected the **In-The-Wild Images** as the **training data**, and the **identity_CelebA, list_attr_celeba, list_bbox_celeba, list_landmarks_celeba as annotation data,** totaling 202,599 images. Among these four types of annotated data, the annotation data for over 200,000 images are stored separately in individual files. 

Therefore, the first step is to configure the "anno_config.toml" file to specify the four types of annotation data to be associated, their category IDs, and the data to be obtained. 

Config the anno_config.toml file.

```toml
title = "celeba anno config"

[[annotation]]
path = "D:/Workplace/internship/mega/tests/mda/test_celeba/anno/identity_CelebA.txt"
start = 1

[[annotation]]
path = "D:/Workplace/internship/mega/tests/mda/test_celeba/anno/list_attr_celeba.txt"
start = 3


[[annotation]]
path = "D:/Workplace/internship/mega/tests/mda/test_celeba/anno/list_bbox_celeba.txt"
start = 3

[[annotation]]
path = "D:/Workplace/internship/mega/tests/mda/test_celeba/anno/list_landmarks_celeba.txt"
start=3
```



### 5.1 Generate MDA Files

Generate MDA files using the following command to combine training data and corresponding annotation data into MDA files:

```shell
cargo run mda --action=generate --mode=combine  --train=tests/mda/test_celeba/train/ --anno-config=tests/mda/test_celeba/anno/anno_config.toml --output=tests/mda/test_celeba/output/  --tags=face  --threads=100
```


Command Explanation:

- action=generate: Generates MDA files.
- mode=combine: Deal the combined annotation data(all annotation data in one file).
- train=tests/mda/test_celeba/train/ :  Path to training data.
- anno-config=tests/mda/test_celeba/anno/anno_config.toml: Path to annotation config file.
- output=tests/mda/test_celeba/output/: Output path for generated MDA files.
- tags=unaligned,face: Tags.
- threads=100: Number of threads.

Output 

```shell
[WARN][2023-09-03 14:32:01] Start to generate mda files...
█████████████████████████████████████████████████████████████████████████████ 202599/202599
[WARN][2023-09-03 14:34:24] 202599 mda files have been generated in 13860.9379484s
```


Note: The running time is determined by the computer's performance.
 

### 5.2 List MDA File Information

**List MDA Anno Groups**
List the annotation groups for one MDA file.

```shell
cargo run mda --action=group --mda=tests/mda/test_celeba/output/000001.mda
```

Command Explanation:

- action=group: List annotation groups.
- mda=tests/mda/test_celeba/output/000001.mda: The path to mda file.

Output 

```shell
+----+-----------------------+
| ID | Annotation Group      |
+----+-----------------------+
| 1  | identity_CelebA       |
+----+-----------------------+
| 2  | list_attr_celeba      |
+----+-----------------------+
| 3  | list_bbox_celeba      |
+----+-----------------------+
| 4  | list_landmarks_celeba |
+----+-----------------------+
```

 There are 4 groups annotation data for this mda file.



**List MDA MetaData and Tags**
List the annotation tags and MetaData for one MDA file.

```shell
cargo run mda --action=list --mda=tests/mda/test_celeba/output/000001.mda
```

Command Explanation:

```shell
action=list: List annotation tags and MetaData.
mda=tests/mda/test_celeba/output/000001.mda: The path to mda file.
```

Output 

```shell
+-----------------------------------------+-------------------+----------------------+------+--------------------------------------------------------------------------+
| MDA File                                | MDA Header Offset | Training Data Offset | Tags | Training MetaData                                                        |
+-----------------------------------------+-------------------+----------------------+------+--------------------------------------------------------------------------+
| tests/mda/test_celeba/output/000001.mda | 188               | 301                  | face | ImageMetaData { size: (409, 687), channel_count: 3, color_space: "RGB" } |
+-----------------------------------------+-------------------+----------------------+------+--------------------------------------------------------------------------+
```

**List Annotation Version**
List the currently available versions of annotation data in MDA files.
Command:

```shell
cargo run mda --action=version --mda=tests/mda/test_celeba/output/000001.mda --group=list_attr_celeba
```

Command Explanation:

- action=version: List version information.
- mda=tests/mda/test_celeba/output/000001.mda : Specified MDA file path.
- group=list_attr_celeba: Specify a specific annotation group.

Output 

```shell
Data Version for "tests/mda/test_celeba/output/000001.mda", anno group: "list_attr_celeba"
+-----+--------+--------+
| rev | offset | length |
+-----+--------+--------+
| 0   | 58861  | 427    |
+-----+--------+--------+
```



Update this mda file and then list the versions
Output

```shell
Data Version for "tests/mda/test_celeba/output/000001.mda", anno group: "list_attr_celeba"
+-----+--------+--------+
| rev | offset | length |
+-----+--------+--------+
| 0   | 58861  | 427    |
+-----+--------+--------+
| 1   | 59288  | 188    |
+-----+--------+--------+
| 2   | 59476  | 86     |
+-----+--------+--------+
```



### 5.3 Update MDA Files

Update the annotation data content in MDA files. I have modified the annotation data for images corresponding to numbers 4, 6, 9, 12, and 17. Now, I want to update the MDA files. I can just specify the program to scan lines 3 to 22 and update the annotation data here.

```shell
cargo run mda --action=update --mda=tests/mda/test_celeba/output/  --anno=D:/Workplace/internship/mega/tests/mda/test_celeba/anno/list_landmarks_celeba.txt --start=3 --end=22 --group=list_attr_celeba
```

Command Explanation:

- action=update: Update MDA content.
- mda=--mda=tests/mda/test_celeba/output/ : Path to the MDA file directory.
- anno=D:/Workplace/internship/mega/tests/mda/test_celeba/anno/list_landmarks_celeba.txt:  Path to the file containing updated annotation data.

- start=3: The line start to read
- end=20: The line end to read 
- group=list_attr_celeba: Specify a specific annotation group.

Output 

```shell
[WARN][2023-09-03 14:59:31] Start to update mda files...
███████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████ 20/20
[WARN][2023-09-03 14:59:40] 10 mda files have been updated in 8.8122533s
```

### 5.4 Extraction from MDA Files

Read data indices and training data information from the MDA files.

```shell
cargo run mda --action=extract --mda=tests/mda/test_celeba/output/000001.mda --train=tests/mda/test_celeba/extract/ --anno=tests/mda/test_celeba/extract/   --group=list_attr_celeba --rev=2
```


Command Explanation:

- action=list: List MDA file information.
- mda=--mda=tests/mda/test_celeba/output/ : Path to the MDA file directory.
- train=tests/mda/test_celeba/extract/ : Path to training data directory.
- anno=tests/mda/test_celeba/extract/:  Path to the file containing updated annotation data.
- group=list_attr_celeba: Specify a specific annotation group.
- rev=2: Get the data where annotation data version=2

Output

```shell
[WARN][2023-09-03 15:01:03] Start to extract mda files...
█████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████ 1/1
[WARN][2023-09-03 15:01:03] 1 mda files have been extracted in 7.8067ms
```



 

## 6 Conclusion

### 6.1 Contribution

In conclusion, .mda file for managing training data and annotations addresses the challenges faced during large-scale model training. By integrating training data, annotation data, and annotation data version information into a single file, we establish a clear correlation between the training data and its annotations, enhancing the overall efficiency and intuitiveness of data management.

### 6.2 Future Work

- Refine the rules for slicing data in version control.
- Optimize the processing speed of a large number of MDA files.
- Provide a search function to the information in the header.
- Provide data version control for training data.

