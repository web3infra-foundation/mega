/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0.
 */

// snippet-start:[rust.example_code.s3.scenario_getting_started.lib]

use aws_sdk_s3::operation::{
    copy_object::{CopyObjectError, CopyObjectOutput},
    create_bucket::{CreateBucketError, CreateBucketOutput},
    get_object::{GetObjectError, GetObjectOutput},
    list_objects_v2::ListObjectsV2Output,
    put_object::{PutObjectError, PutObjectOutput},
};
use aws_sdk_s3::types::{
    BucketLocationConstraint, CreateBucketConfiguration, Delete, ObjectIdentifier,
};
use aws_sdk_s3::{error::SdkError, primitives::ByteStream, Client};
use error::Error;
use std::path::Path;
use std::str;

pub mod error;

// snippet-start:[rust.example_code.s3.basics.delete_bucket]
pub async fn delete_bucket(client: &Client, bucket_name: &str) -> Result<(), Error> {
    client.delete_bucket().bucket(bucket_name).send().await?;
    println!("Bucket deleted");
    Ok(())
}
// snippet-end:[rust.example_code.s3.basics.delete_bucket]

// snippet-start:[rust.example_code.s3.basics.delete_objects]
pub async fn delete_objects(client: &Client, bucket_name: &str) -> Result<Vec<String>, Error> {
    let objects = client.list_objects_v2().bucket(bucket_name).send().await?;

    let mut delete_objects: Vec<ObjectIdentifier> = vec![];
    for obj in objects.contents() {
        let obj_id = ObjectIdentifier::builder()
            .set_key(Some(obj.key().unwrap().to_string()))
            .build()
            .map_err(Error::from)?;
        delete_objects.push(obj_id);
    }

    let return_keys = delete_objects.iter().map(|o| o.key.clone()).collect();

    client
        .delete_objects()
        .bucket(bucket_name)
        .delete(
            Delete::builder()
                .set_objects(Some(delete_objects))
                .build()
                .map_err(Error::from)?,
        )
        .send()
        .await?;

    let objects: ListObjectsV2Output = client.list_objects_v2().bucket(bucket_name).send().await?;

    eprintln!("{objects:?}");

    match objects.key_count.unwrap_or_default() {
        0 => Ok(return_keys),
        _ => Err(Error::unhandled(
            "There were still objects left in the bucket.",
        )),
    }
}
// snippet-end:[rust.example_code.s3.basics.delete_objects]

// snippet-start:[rust.example_code.s3.basics.list_objects]
pub async fn list_objects(client: &Client, bucket_name: &str) -> Result<(), Error> {
    let objects = client.list_objects_v2().bucket(bucket_name).send().await?;
    println!("Objects in bucket:");
    for obj in objects.contents() {
        println!("{:?}", obj.key().unwrap());
    }

    Ok(())
}
// snippet-end:[rust.example_code.s3.basics.list_objects]

// snippet-start:[rust.example_code.s3.basics.copy_object]
pub async fn copy_object(
    client: &Client,
    bucket_name: &str,
    object_key: &str,
    target_key: &str,
) -> Result<CopyObjectOutput, SdkError<CopyObjectError>> {
    let mut source_bucket_and_object: String = "".to_owned();
    source_bucket_and_object.push_str(bucket_name);
    source_bucket_and_object.push('/');
    source_bucket_and_object.push_str(object_key);

    client
        .copy_object()
        .copy_source(source_bucket_and_object)
        .bucket(bucket_name)
        .key(target_key)
        .send()
        .await
}
// snippet-end:[rust.example_code.s3.basics.copy_object]

// snippet-start:[rust.example_code.s3.basics.download_object]
pub async fn download_object(
    client: &Client,
    bucket_name: &str,
    key: &str,
) -> Result<GetObjectOutput, SdkError<GetObjectError>> {
    client
        .get_object()
        .bucket(bucket_name)
        .key(key)
        .send()
        .await
}
// snippet-end:[rust.example_code.s3.basics.download_object]

// snippet-start:[rust.example_code.s3.basics.upload_object]
// snippet-start:[rust.example_code.s3.basics.put_object]
pub async fn upload_object_from_content(
    client: &Client,
    bucket_name: &str,
    content: &[u8],
    key: &str,
) -> Result<PutObjectOutput, SdkError<PutObjectError>> {
    let body = ByteStream::from(content.to_owned());
    client
        .put_object()
        .bucket(bucket_name)
        .key(key)
        .body(body)
        .send()
        .await
}

pub async fn upload_object(
    client: &Client,
    bucket_name: &str,
    file_name: &str,
    key: &str,
) -> Result<PutObjectOutput, SdkError<PutObjectError>> {
    let body = ByteStream::from_path(Path::new(file_name)).await;
    client
        .put_object()
        .bucket(bucket_name)
        .key(key)
        .body(body.unwrap())
        .send()
        .await
}
// snippet-end:[rust.example_code.s3.basics.put_object]
// snippet-end:[rust.example_code.s3.basics.upload_object]

// snippet-start:[rust.example_code.s3.basics.create_bucket]
pub async fn create_bucket(
    client: &Client,
    bucket_name: &str,
    region: &str,
) -> Result<CreateBucketOutput, SdkError<CreateBucketError>> {
    let constraint = BucketLocationConstraint::from(region);
    let cfg = CreateBucketConfiguration::builder()
        .location_constraint(constraint)
        .build();
    client
        .create_bucket()
        .create_bucket_configuration(cfg)
        .bucket(bucket_name)
        .send()
        .await
}
// snippet-end:[rust.example_code.s3.basics.create_bucket]
// snippet-end:[rust.example_code.s3.scenario_getting_started.lib]
