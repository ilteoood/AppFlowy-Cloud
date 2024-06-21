use client_api::entity::{PublishCollabItem, PublishCollabMetadata};
use client_api_test::{generate_unique_registered_user_client, localhost_client};

#[tokio::test]
async fn test_set_publish_namespace_set() {
  let (c, _user) = generate_unique_registered_user_client().await;
  let workspace_id = get_first_workspace_string(&c).await;

  {
    // cannot get namespace if not set
    let err = c
      .get_workspace_publish_namespace(&workspace_id.to_string())
      .await
      .err()
      .unwrap();
    assert_eq!(format!("{:?}", err.code), "PublishNamespaceNotSet");
  }

  let namespace = uuid::Uuid::new_v4().to_string();
  c.set_workspace_publish_namespace(&workspace_id.to_string(), &namespace)
    .await
    .unwrap();

  {
    // cannot set the same namespace
    let err = c
      .set_workspace_publish_namespace(&workspace_id.to_string(), &namespace)
      .await
      .err()
      .unwrap();
    assert_eq!(format!("{:?}", err.code), "PublishNamespaceAlreadyTaken");
  }
  {
    // can replace the namespace
    let namespace = uuid::Uuid::new_v4().to_string();
    c.set_workspace_publish_namespace(&workspace_id.to_string(), &namespace)
      .await
      .unwrap();

    let got_namespace = c
      .get_workspace_publish_namespace(&workspace_id.to_string())
      .await
      .unwrap();
    assert_eq!(got_namespace, namespace);
  }
  {
    // cannot set namespace too short
    let err = c
      .set_workspace_publish_namespace(&workspace_id.to_string(), "a") // too short
      .await
      .err()
      .unwrap();
    assert_eq!(format!("{:?}", err.code), "InvalidRequest");
  }

  {
    // cannot set namespace with invalid chars
    let err = c
      .set_workspace_publish_namespace(&workspace_id.to_string(), "/|(*&)(&#@!") // invalid chars
      .await
      .err()
      .unwrap();
    assert_eq!(format!("{:?}", err.code), "InvalidRequest");
  }
}

#[tokio::test]
async fn test_publish_doc() {
  let (c, _user) = generate_unique_registered_user_client().await;
  let workspace_id = get_first_workspace_string(&c).await;
  let my_namespace = uuid::Uuid::new_v4().to_string();
  c.set_workspace_publish_namespace(&workspace_id.to_string(), &my_namespace)
    .await
    .unwrap();

  let doc_name_1 = "doc1";
  let view_id_1 = uuid::Uuid::new_v4();
  let doc_name_2 = "doc2";
  let view_id_2 = uuid::Uuid::new_v4();
  c.publish_collabs::<MyCustomMetadata, &[u8]>(
    &workspace_id,
    vec![
      PublishCollabItem {
        meta: PublishCollabMetadata {
          view_id: view_id_1,
          doc_name: doc_name_1.to_string(),
          metadata: MyCustomMetadata {
            title: "my_title_1".to_string(),
          },
        },
        data: "yrs_encoded_data_1".as_bytes(),
      },
      PublishCollabItem {
        meta: PublishCollabMetadata {
          view_id: view_id_2,
          doc_name: doc_name_2.to_string(),
          metadata: MyCustomMetadata {
            title: "my_title_2".to_string(),
          },
        },
        data: "yrs_encoded_data_2".as_bytes(),
      },
    ],
  )
  .await
  .unwrap();

  {
    // Non login user should be able to view the published collab
    let guest_client = localhost_client();
    let published_collab = guest_client
      .get_published_collab::<MyCustomMetadata>(&my_namespace, doc_name_1)
      .await
      .unwrap();
    assert_eq!(published_collab.title, "my_title_1");

    let publish_info = guest_client
      .get_published_collab_info(&view_id_1)
      .await
      .unwrap();
    assert_eq!(publish_info.namespace, Some(my_namespace.clone()));
    assert_eq!(publish_info.doc_name, doc_name_1);
    assert_eq!(publish_info.view_id, view_id_1);

    let blob = guest_client
      .get_published_collab_blob(&my_namespace, doc_name_1)
      .await
      .unwrap();
    assert_eq!(blob, "yrs_encoded_data_1");
  }

  c.unpublish_collabs(&workspace_id, &[view_id_1])
    .await
    .unwrap();

  {
    // Deleted collab should not be accessible
    let guest_client = localhost_client();
    let err = guest_client
      .get_published_collab::<MyCustomMetadata>(&my_namespace, doc_name_1)
      .await
      .err()
      .unwrap();
    assert_eq!(format!("{:?}", err.code), "RecordNotFound");

    let guest_client = localhost_client();
    let err = guest_client
      .get_published_collab_blob(&my_namespace, doc_name_1)
      .await
      .err()
      .unwrap();
    assert_eq!(format!("{:?}", err.code), "RecordNotFound");
  }
}

#[tokio::test]
async fn test_publish_load_test() {
  let (c, _user) = generate_unique_registered_user_client().await;
  let workspace_id = get_first_workspace_string(&c).await;
  let my_namespace = uuid::Uuid::new_v4().to_string();
  c.set_workspace_publish_namespace(&workspace_id.to_string(), &my_namespace)
    .await
    .unwrap();

  // publish nothing
  c.publish_collabs::<(), &[u8]>(&workspace_id, vec![])
    .await
    .unwrap();

  // publish nothing with metadata
  c.publish_collabs::<MyCustomMetadata, &[u8]>(&workspace_id, vec![])
    .await
    .unwrap();

  // publish 1000 collabs
  let collabs: Vec<PublishCollabItem<MyCustomMetadata, Vec<u8>>> = (0..1000)
    .map(|i| PublishCollabItem {
      meta: PublishCollabMetadata {
        view_id: uuid::Uuid::new_v4(),
        doc_name: format!("doc{}", i),
        metadata: MyCustomMetadata {
          title: format!("title{}", i),
        },
      },
      data: vec![0; 100_000], // 100 KB
    })
    .collect();

  c.publish_collabs(&workspace_id, collabs).await.unwrap();
}

async fn get_first_workspace_string(c: &client_api::Client) -> String {
  c.get_workspaces()
    .await
    .unwrap()
    .0
    .first()
    .unwrap()
    .workspace_id
    .to_string()
}

#[derive(serde::Serialize, serde::Deserialize)]
struct MyCustomMetadata {
  title: String,
}
