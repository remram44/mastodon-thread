use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::Error;

#[derive(Deserialize)]
struct ActivityPubToot {
    id: String,
    published: String,
    #[serde(rename = "attributedTo")]
    attributed_to: String,
    content: String,
    #[serde(rename = "inReplyTo")]
    in_reply_to: Option<String>,
}

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
            message: toot.content,
        }
    }
}

pub struct Thread {
    pub toot: Toot,
    pub children: Vec<Arc<Mutex<Thread>>>,
}

pub async fn load_thread(client: reqwest::Client, target_url: &str) -> Result<Thread, Error> {
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
    let first_url = toot.url.clone();

    // Get the replies URL
    let page_url = res
        .get("replies")
        .and_then(|r| r.get("first"))
        .and_then(|r| r.get("next"));
    let mut page_url = match page_url {
        Some(serde_json::Value::String(s)) => s.to_owned(),
        _ => return Err(Error::Other("Missing replies link".into())),
    };

    // Create top-level thread
    let thread = Arc::new(Mutex::new(Thread {
        toot,
        children: Vec::new(),
    }));

    // Map toot IDs to their Thread, to insert replies
    let mut toot_map: HashMap<String, Arc<Mutex<Thread>>> = HashMap::new();
    toot_map.insert(first_url, thread.clone());

    // Load the replies, which might spawn multiple pages
    loop {
        eprintln!("Getting page of replies {}", page_url);
        let mut res = client
            .get(page_url)
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
            if let serde_json::Value::String(_) = item {
                // Skip
                eprintln!("Is URL, skip");
                continue;
            }
            let item: ActivityPubToot = serde_json::from_value(item)?;

            // Find parent thread
            let mut parent_thread = thread.clone();
            if let Some(ref parent_id) = item.in_reply_to.as_ref() {
                let parent_id: &str = parent_id;
                if let Some(t) = toot_map.get(parent_id) {
                    parent_thread = t.clone();
                }
            }

            // Create new entry
            let toot: Toot = item.into();
            let toot_url = toot.url.clone();
            let new_thread = Arc::new(Mutex::new(Thread {
                toot,
                children: Vec::new(),
            }));

            // Put it in the map
            eprintln!("Inserting {}", toot_url);
            toot_map.insert(toot_url, new_thread.clone());

            // Insert into parent
            parent_thread.lock().unwrap().children.push(new_thread);
        }

        match res.get("next") {
            Some(serde_json::Value::String(url)) => page_url = url.to_owned(),
            _ => break,
        }
    }

    eprintln!("Done getting replies");
    drop(toot_map);
    Ok(Arc::try_unwrap(thread).map_err(|_| "").unwrap().into_inner().unwrap())
}
