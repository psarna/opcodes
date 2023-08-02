use anyhow::{Context, Result};
use scraper::{Html, Selector};
use spin_sdk::{
    http::{Request, Response},
    http_component,
};

fn fetch_opcode_info(opcode_name: &str, exact_match: bool) -> Result<Option<bytes::Bytes>> {
    let req = http::Request::builder().uri("https://sqlite.org/opcode.html");
    let resp = spin_sdk::outbound_http::send_request(req.body(None)?)?;
    let resp = resp.into_body().context("Error fetching the webpage")?;

    let html_content = std::str::from_utf8(&resp)?;
    let document = Html::parse_document(html_content);

    // Find the div with class "optab"
    let optab_selector = Selector::parse("div.optab").unwrap();
    let optab_div = document.select(&optab_selector).next();
    let mut concatenated_infos = String::new();

    if let Some(optab_div) = optab_div {
        // Find all the tables within the "optab" div
        let table_selector = Selector::parse("table").unwrap();
        let tables = optab_div.select(&table_selector);

        for table in tables {
            println!("Checking table");
            // Find all the rows in the table
            let row_selector = Selector::parse("tr").unwrap();
            let rows = table.select(&row_selector);

            for row in rows {
                // Workaround for buggy <td> tags in the SQLite webpage
                let a_selector = Selector::parse("a").unwrap();
                let a_tags = row.select(&a_selector).collect::<Vec<_>>();

                if !a_tags.is_empty() {
                    let opcode = a_tags[0].value().attr("name").unwrap().trim();
                    let matches = if exact_match {
                        opcode.to_lowercase() == opcode_name.to_lowercase()
                    } else {
                        opcode.to_lowercase().contains(&opcode_name.to_lowercase())
                    };

                    if matches {
                        println!("Match found: {opcode} for {opcode_name}");
                        // Find the columns (td tags) in the row
                        let column_selector = Selector::parse("td").unwrap();
                        let columns = row.select(&column_selector).collect::<Vec<_>>();
                        let info = columns[1].inner_html().trim().to_string();
                        if exact_match {
                            return Ok(Some(info.into()));
                        }
                        concatenated_infos += &format!("Opcode: {opcode}\n{info}\n\n");
                    }
                } else {
                    println!("a_tags empty");
                }
            }
        }
    } else {
        println!("div.optab not found");
    }

    if exact_match {
        return Ok(None);
    }
    Ok(Some(concatenated_infos.into()))
}

/// A simple Spin HTTP component.
#[http_component]
fn handle_opcodes(req: Request) -> Result<Response> {
    let opcode = req
        .uri()
        .path()
        .trim_start_matches('/')
        .trim_end_matches('/')
        .to_lowercase();
    println!("Opcode {opcode}");
    let builder = http::Response::builder().status(200);
    let opcode_info = match opcode.as_str() {
        "" => {
            return Ok(builder.body(Some(
                "Please specify a libSQL/SQLite opcode in the path, e.g. /init".into(),
            ))?)
        }
        "favicon.ico" => return Ok(builder.body(None)?),
        _ => match fetch_opcode_info(&opcode, true)? {
            Some(info) => Some(info),
            // Retry with fuzzy search if an exact match is not found
            None => fetch_opcode_info(&opcode, false)?,
        },
    };
    Ok(builder.body(opcode_info)?)
}
