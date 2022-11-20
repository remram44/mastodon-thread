use async_recursion::async_recursion;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use super::Error;

#[derive(Deserialize)]
struct ActivityPubToot {
    id: String,
    published: String,
    #[serde(rename = "attributedTo")]
    attributed_to: String,
    content: String,
}

#[derive(Serialize)]
pub struct Toot {
    pub url: String,
    pub author: String,
    pub message: String,
}

impl From<ActivityPubToot> for Toot {
    fn from(toot: ActivityPubToot) -> Toot {
        Toot {
            url: toot.id,
            author: toot.attributed_to,
            message: ammonia::clean(&toot.content),
        }
    }
}

#[derive(Serialize)]
pub struct Thread {
    pub toot: Toot,
    pub children: Vec<Arc<Mutex<Thread>>>,
}

pub async fn load_thread(client: reqwest::Client, target_url: &str) -> Result<Arc<Mutex<Thread>>, Error> {
    // Load the provided toot
    eprintln!("Getting toot {}", target_url);
    let res = client
        .get(target_url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    // Get that first toot
    let toot: ActivityPubToot = serde_json::from_value(res.clone())?;
    let toot: Toot = toot.into();

    // Get the replies URL
    let replies_page_url = res
        .get("replies")
        .and_then(|r| r.get("first"))
        .and_then(|r| r.get("next"));
    let replies_page_url = match replies_page_url {
        Some(serde_json::Value::String(s)) => s.to_owned(),
        _ => return Err(Error::Other("Missing replies link".into())),
    };

    // Create top-level thread
    let thread = Arc::new(Mutex::new(Thread {
        toot,
        children: Vec::new(),
    }));

    load_replies(client, thread.clone(), replies_page_url).await?;

    eprintln!("Done getting toot");
    Ok(thread)
}

#[async_recursion]
async fn load_replies(
    client: reqwest::Client,
    thread: Arc<Mutex<Thread>>,
    mut replies_page_url: String,
) -> Result<(), Error> {
    loop {
        eprintln!("Getting page of replies {}", replies_page_url);
        let mut res = client
            .get(replies_page_url)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let Some(serde_json::Value::Array(items)) = res
            .get_mut("items")
            .map(serde_json::Value::take)
        else {
            return Err(Error::Other("Invalid replies data".into()));
        };

        for item in items {
            eprintln!("Reading item");
            let new_thread = if let serde_json::Value::String(url) = item {
                // Load thread from toot
                load_thread(client.clone(), &url).await?
            } else {
                // Get the replies URL
                let new_replies = item
                    .get("replies")
                    .and_then(|r| r.get("first"))
                    .and_then(|r| r.get("next"));
                let new_replies = match new_replies {
                    Some(serde_json::Value::String(s)) => Some(s.to_owned()),
                    _ => None,
                };

                let item: ActivityPubToot = serde_json::from_value(item)?;

                // Create new entry
                let new_thread = Arc::new(Mutex::new(Thread {
                    toot: item.into(),
                    children: Vec::new(),
                }));

                // Fill it recursively
                if let Some(new_replies) = new_replies {
                    load_replies(client.clone(), new_thread.clone(), new_replies).await?;
                }

                new_thread
            };

            // Insert into parent
            thread.lock().unwrap().children.push(new_thread);
        }

        match res.get("next") {
            Some(serde_json::Value::String(url)) => replies_page_url = url.to_owned(),
            _ => break,
        }
    }

    eprintln!("No more pages of replies");

    Ok(())
}
