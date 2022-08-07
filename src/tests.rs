use std::{sync::Arc, thread, time::Duration};

use cdds_derive::Topic;

use cyclonedds_rs::*;

use crate::{AsyncReader, NodeBuilder, PublishOptions, Samples, SubscribeOptions, qos::QosReliability};

#[derive(Topic, Deserialize, Serialize)]
struct SenderType {
    pub msg1: String,
    pub msg2: String,
}

#[derive(Topic, Deserialize, Serialize)]
struct ResponseType {
    pub msg1: String,
    pub msg2 : String,
}

#[test]
fn test_pub_sub_simple() {
    thread::spawn(|| {
        let node = NodeBuilder::default()
            .build("testnode".to_string())
            .unwrap();
        let publish_options = PublishOptions::default();
        let mut writer = node.advertise::<SenderType>(&publish_options).unwrap();
        let sub_options = SubscribeOptions::default();

        let mut reader = node.subscribe_async::<ResponseType>(&sub_options).unwrap();

        let terminate_handle = node.clone();

        node.spin(|| {
            tokio::spawn(async move {
                //loop {
                let msg = SenderType {
                    msg1: "message1".to_owned(),
                    msg2: "message2".to_owned(),
                };
                println!("Publishing");
                writer.publish(Arc::new(msg)).unwrap();
                async_std::task::sleep(Duration::from_millis(100)).await;

                let mut rx_samples = Samples::<ResponseType>::new(1);
                let num = reader.take(&mut rx_samples).await.unwrap();

                let msg = rx_samples.iter().take(1).next().unwrap();
                assert_eq!(msg.msg1,"message1".to_owned());
                assert_eq!(msg.msg2,"message2".to_owned());

                terminate_handle.terminate();
                //}
            });
        })
        .unwrap();
    });

    // subscriber node

    let node = NodeBuilder::default()
        .build("testrxnode".to_owned())
        .unwrap();
    let sub_options = SubscribeOptions::default();
    let publish_options = PublishOptions::default();

    let mut reader = node.subscribe_async::<SenderType>(&sub_options).unwrap();
    let mut writer = node.advertise::<ResponseType>(&publish_options).unwrap();

    let terminate_handle = node.clone();

    node.spin(move || {
        tokio::spawn(async move {
            let mut samples = Samples::<SenderType>::new(1);
            
            //loop {
            let num_message = reader.take(&mut samples).await.unwrap();

            let msg = samples.iter().take(1).next().unwrap();
            println!("Got: {} : {}", msg.msg1, msg.msg2);
            assert_eq!(msg.msg1,"message1".to_owned());
            assert_eq!(msg.msg2,"message2".to_owned());


            let rx = Arc::new(ResponseType {
                msg1: msg.msg1.clone(),
                msg2: msg.msg2.clone(),
            });

            writer.publish(rx).unwrap();


            terminate_handle.terminate();
            //}
        });
        println!("Rx ended");
    })
    .unwrap();
}
