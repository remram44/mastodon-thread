use serde::Deserialize;

use super::Error;

#[derive(Deserialize)]
struct ActivityPubToot {
    id: String,
    published: String,
    #[serde(rename = "attributedTo")]
    attributed_to: String,
    content: String,
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
    pub children: Vec<Thread>,
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
    let toot = toot.into();

    // Get the replies URL
    let page_url = res
        .get("replies")
        .and_then(|r| r.get("first"))
        .and_then(|r| r.get("next"));
    let mut page_url = match page_url {
        Some(serde_json::Value::String(s)) => s.to_owned(),
        _ => return Err(Error::Other("Missing replies link".into())),
    };

    let mut thread = Thread {
        toot,
        children: Vec::new(),
    };

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
            thread.children.push(Thread {
                toot: item.into(),
                children: Vec::new(),
            });
        }

        match res.get("next") {
            Some(serde_json::Value::String(url)) => page_url = url.to_owned(),
            _ => break,
        }
    }

    eprintln!("Done getting replies");
    Ok(thread)
}
