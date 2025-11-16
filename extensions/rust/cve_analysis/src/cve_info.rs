use crate::{model::CveAnalyzeRes, RustsecInfo};
use data_transporter::{data_reader::{DataReader, DataReaderTrait}, db::{ DBHandler}};
use regex::Regex;
//use reqwest;
use scraper::{Html, Selector};
pub async fn get_cve_info(id:String,url:String)->Result<RustsecInfo,Box<dyn std::error::Error>>{
        tracing::info!("Getting info {} in {}",id.clone(),url.clone());
        let resp = reqwest::get(&url).await?;

        if !resp.status().is_success() {
            tracing::error!("HTTP request failed: {}", resp.status());
            return Err("HTTP request failed".into());
        }

        let body = resp.text().await?;

        let document = Html::parse_document(&body);
        let subtitle_selector = Selector::parse("span.subtitle>p").unwrap();
        let reported_selector = Selector::parse("dl>dt#reported+dd").unwrap();
        let issued_selector = Selector::parse("dl>dt#issued+dd").unwrap();
        let package_selector = Selector::parse("dl>dt#package+dd").unwrap();
        let type_selector = Selector::parse("dl>dt#type+dd").unwrap();
        let keywords_selector = Selector::parse("dl>dt#keywords+dd").unwrap();
        let aliases_selector = Selector::parse("dl>dt#aliases+dd").unwrap();
        let details_selector = Selector::parse("dl>dt#details+dd").unwrap();
        let patched_selector = Selector::parse("dl>dt#patched+dd").unwrap();
        let unaffected_selector = Selector::parse("dl>dt#unaffected+dd").unwrap();
        let description_selector = Selector::parse("h3#description+p").unwrap();
        let dt_selector = Selector::parse("dl dt").unwrap();
        let dd_selector = Selector::parse("dl dd").unwrap();
        let mut subtitle = "".to_string();
        let mut reported = "".to_string();
        let mut issued = "".to_string();
        let mut package = "".to_string();
        let mut ttype = "".to_string();
        let mut keywords = "".to_string();
        let mut aliases = "".to_string();
        let mut reference = "".to_string();
        let mut patched = "".to_string();
        let mut unaffected = "".to_string();
        let mut description = "".to_string();
        let mut affected_function = "".to_string();
        //let mut affected_version = "".to_string();
        let mut affected_res = vec![];
        for element in document.select(&subtitle_selector) {
            let text = element.text().collect::<Vec<_>>().join("");
            let res = text
                .trim_start_matches(|c: char| c.is_whitespace())
                .trim_end_matches(|c: char| c.is_whitespace());
            let real_res = res.split_whitespace().collect::<Vec<&str>>().join(" ");
            subtitle = real_res.clone();
            //println!("subtitle: {}",real_res);
        }
        for element in document.select(&reported_selector) {
            let text = element.text().collect::<Vec<_>>().join("");
            let res = text
                .trim_start_matches(|c: char| c.is_whitespace())
                .trim_end_matches(|c: char| c.is_whitespace());
            let real_res = res.split_whitespace().collect::<Vec<&str>>().join(" ");
            reported = real_res.clone();
            //println!("reported: {}",real_res);
        }
        for element in document.select(&issued_selector) {
            let text = element.text().collect::<Vec<_>>().join("");
            let res = text
                .trim_start_matches(|c: char| c.is_whitespace())
                .trim_end_matches(|c: char| c.is_whitespace());
            let real_res = res.split_whitespace().collect::<Vec<&str>>().join(" ");
            issued = real_res.clone();
            //println!("issued: {}",real_res);
        }
        for element in document.select(&package_selector) {
            let text = element.text().collect::<Vec<_>>().join("");
            let res = text
                .trim_start_matches(|c: char| c.is_whitespace())
                .trim_end_matches(|c: char| c.is_whitespace());
            let real_res = res.split_whitespace().collect::<Vec<&str>>().join(" ");
            package = real_res.clone();
            //println!("package: {}",real_res);
        }
        for element in document.select(&type_selector) {
            let text = element.text().collect::<Vec<_>>().join("");
            let res = text
                .trim_start_matches(|c: char| c.is_whitespace())
                .trim_end_matches(|c: char| c.is_whitespace());
            let real_res = res.split_whitespace().collect::<Vec<&str>>().join(" ");
            ttype = real_res.clone();
            //println!("type: {}",real_res);
        }
        for element in document.select(&keywords_selector) {
            let text = element.text().collect::<Vec<_>>().join("");
            let res = text
                .trim_start_matches(|c: char| c.is_whitespace())
                .trim_end_matches(|c: char| c.is_whitespace());
            let real_res = res.split_whitespace().collect::<Vec<&str>>().join(" ");
            keywords = real_res.clone();
            //println!("keywords: {}",real_res);
        }
        for element in document.select(&aliases_selector) {
            let text = element.text().collect::<Vec<_>>().join("");
            let res = text
                .trim_start_matches(|c: char| c.is_whitespace())
                .trim_end_matches(|c: char| c.is_whitespace());
            let real_res = res.split_whitespace().collect::<Vec<&str>>().join(" ");
            aliases = real_res.clone();
            //println!("aliases: {}",real_res);
        }
        for element in document.select(&details_selector) {
            let text = element.text().collect::<Vec<_>>().join("");
            let res = text
                .trim_start_matches(|c: char| c.is_whitespace())
                .trim_end_matches(|c: char| c.is_whitespace());
            let real_res = res.split_whitespace().collect::<Vec<&str>>().join(" ");
            reference = real_res.clone();
            //println!("details: {}",real_res);
        }
        for element in document.select(&patched_selector) {
            let text = element.text().collect::<Vec<_>>().join("");
            let res = text
                .trim_start_matches(|c: char| c.is_whitespace())
                .trim_end_matches(|c: char| c.is_whitespace());
            let real_res = res.split_whitespace().collect::<Vec<&str>>().join(" ");
            patched = real_res.clone();
            //println!("patched: {}",real_res);
        }
        for element in document.select(&unaffected_selector) {
            let text = element.text().collect::<Vec<_>>().join("");
            let res = text
                .trim_start_matches(|c: char| c.is_whitespace())
                .trim_end_matches(|c: char| c.is_whitespace());
            let real_res = res.split_whitespace().collect::<Vec<&str>>().join(" ");
            unaffected = real_res.clone();
            //println!("unaffected: {}",real_res);
        }
        for element in document.select(&description_selector) {
            let text = element.text().collect::<Vec<_>>().join("");
            let res = text
                .trim_start_matches(|c: char| c.is_whitespace())
                .trim_end_matches(|c: char| c.is_whitespace());
            let real_res = res.split_whitespace().collect::<Vec<&str>>().join(" ");
            description = real_res.clone();
            //println!("description: {}",real_res);
        }
        let dt_elements: Vec<_> = document.select(&dt_selector).collect();
        let dd_elements: Vec<_> = document.select(&dd_selector).collect();
        for (i, dt_element) in dt_elements.iter().enumerate() {
            let dt_text = dt_element.text().collect::<String>();
            let trimmed_dt = dt_text.trim();
            
            if trimmed_dt == "Affected Functions" {
                if i < dd_elements.len() {
                    //let dd_text = dd_elements[i].text().collect::<String>();
                    //let trimmed_dd = dd_text.trim();
                }
                for j in (i + 1)..dt_elements.len() {
                    let next_dt = &dt_elements[j];
                    //let next_dt_text = next_dt.text().collect::<String>();
                    //let trimmed_next_dt = next_dt_text.trim();
                    let code_selector = Selector::parse("code").unwrap();
                    let mut has_function = false;
                    for code_element in next_dt.select(&code_selector) {
                        let function_name = code_element.text().collect::<String>();
                        let trimmed_function = function_name.trim();
                        if !trimmed_function.is_empty() {
                            affected_function = trimmed_function.to_string();
                            has_function = true;
                        }
                    }
                    if has_function && j < dd_elements.len() {
                        let version_dd = &dd_elements[j];
                        let li_selector = Selector::parse("ul li code").unwrap();
                        let mut versions = Vec::new();
                        for version_element in version_dd.select(&li_selector) {
                            let version = version_element.text().collect::<String>();
                            let trimmed_version = version.trim();
                            if !trimmed_version.is_empty() {
                                versions.push(trimmed_version.to_string());
                            }
                        }
                        if !versions.is_empty() {
                            for version in versions {
                                let one_affected = affected_function.clone()+" "+&version;
                                affected_res.push(one_affected);
                            }
                        }
                    }
                }
                break;
            }
        }
        let affected = affected_res.join(",");
        let res_info = RustsecInfo {
            id: id.clone(),
            subtitle,
            reported,
            issued,
            package,
            ttype,
            keywords,
            aliases,
            reference,
            patched,
            unaffected,
            description,
            affected,
        };
        tracing::info!("Finish getting info {} in {}",id.clone(),url.clone());
        Ok(res_info)
}
#[allow(clippy::never_loop)]
pub async fn analyze_cve(datareader:&DataReader,dbhandler:&DBHandler,id:String)->Result<(),Box<dyn std::error::Error>>{
    tracing::info!("start analyze cve:{}",id.clone());
    let rows = dbhandler.client.query("SELECT * FROM rustsec_info WHERE id = $1;",&[&id] ).await.expect("failed to query rustsec_info");
    let mut cve_info = vec![];
    for row in rows{
        let subtitle:String = row.get("subtitle");
        let reported:String = row.get("reported");
        let issued:String = row.get("issued");
        let package:String = row.get("package");
        let ttype:String = row.get("type");
        let keywords:String = row.get("keywords");
        let aliases:String = row.get("aliases");
        let reference:String = row.get("reference");
        let patched:String = row.get("patched");
        let unaffected:String = row.get("unaffected");
        let description:String = row.get("description");
        let affected:String = row.get("affected");
        let info = RustsecInfo{ 
            id:id.clone(),
            subtitle, 
            reported, 
            issued, 
            package, 
            ttype, 
            keywords, 
            aliases, 
            reference, 
            patched, 
            unaffected, 
            description,
            affected,};
        cve_info.push(info.clone());
        break;
    }
    let re2 = Regex::new(r"^[A-Za-z0-9_-]+").expect("Failed to create regex");
    for info in cve_info.clone(){
        let patched: String = if info.clone().patched.starts_with("no") {
            String::new()
        } else {
            info.clone().patched
        };
        let sep = " "; 
        let combined = [&patched, &info.unaffected]
            .into_iter()
            .map(|s| s.as_str())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(sep);
        
    // 按匹配到的空格拆分，并收集结果
        let items:Vec<String> = split_constraints_replace(&combined);
        let crate_name = re2.find(&info.package).map(|m| m.as_str()).unwrap_or("").to_string();
        tracing::info!("name:{},patched:{}",crate_name.clone(),combined.clone());
        let namespace = format!("crates/{}",crate_name.clone());
        let all_versions = datareader.new_get_lib_version(namespace.clone(), crate_name.clone()).await.expect("failed to get all versions;");
        for version in all_versions.clone(){
            tracing::info!("all versions:{}",version.clone());
        }
        let patched_string: String = items
        .iter()
        .filter(|s| !s.is_empty())
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join("|");
        tracing::info!("patched_string:{}",patched_string.clone());
        let mut matched_versions = vec![];
        for version in all_versions.clone(){
            let matched = dbhandler.match_version(patched_string.clone(), version.clone()).await.expect("failed to match version");
            if !matched{
                matched_versions.push(version.clone());
            }
        }
        for version in matched_versions.clone(){
            tracing::info!("matched versions:{}",version.clone());
        }
        let mut all_res = vec![];
        for version in matched_versions.clone() {
            let name_and_version = crate_name.clone()+"/"+&version;
            let dependents = datareader.new_get_direct_dependent_nodes(&namespace, &name_and_version).await.unwrap();
            let mut all_depts = vec![];
            for dep in dependents{
                let nv = dep.name+"/"+&dep.version;
                all_depts.push(nv);
            }
            let one_res = CveAnalyzeRes{ crate_version: name_and_version.clone(), dept_crate_version: all_depts.clone() };
            all_res.push(one_res);
            let depts:String = serde_json::to_string(&all_depts).expect("failed to serialize depts");
            tracing::info!("crate:{},version:{},depts:{}",crate_name.clone(),version.clone(),depts.clone());
        }
        let value: String = serde_json::to_string(&all_res).expect("failed to serialize all_res");
        tracing::info!("value:{}",value.clone());
        dbhandler.client.execute("INSERT INTO cve_analysis_res(id,res) VALUES ($1,$2) 
                                            ON CONFLICT (id) DO UPDATE SET 
                                            res=EXCLUDED.res;", &[&id,&value]).await.expect("failed to insert/update cve_analysis_res");
    }

    Ok(())
}
fn split_constraints_replace(s: &str) -> Vec<String> {
    let marker = "\u{FFFF}"; // 很少用到的占位
    let protected = s.replace(", ", marker);
    protected
        .split(' ')
        .filter(|t| !t.is_empty())
        .map(|t| t.replace(marker, ", "))
        .collect()
}