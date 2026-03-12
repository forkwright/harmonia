pub const MIME_OPDS_V1: &str = "application/atom+xml;profile=opds-catalog;charset=utf-8";
pub const MIME_OPENSEARCH: &str = "application/opensearchdescription+xml";

pub struct AtomLink {
    pub rel: String,
    pub href: String,
    pub link_type: String,
    pub title: Option<String>,
}

pub struct AtomEntry {
    pub id: String,
    pub title: String,
    pub updated: String,
    pub summary: Option<String>,
    pub links: Vec<AtomLink>,
}

pub struct AtomFeed {
    pub id: String,
    pub title: String,
    pub updated: String,
    pub links: Vec<AtomLink>,
    pub entries: Vec<AtomEntry>,
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

impl AtomFeed {
    pub fn to_xml(&self) -> String {
        let mut out = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <feed xmlns=\"http://www.w3.org/2005/Atom\"\
               xmlns:opds=\"http://opds-spec.org/2010/catalog\"\
               xmlns:dc=\"http://purl.org/dc/terms/\">\n",
        );
        out.push_str(&format!("  <id>{}</id>\n", escape_xml(&self.id)));
        out.push_str(&format!("  <title>{}</title>\n", escape_xml(&self.title)));
        out.push_str(&format!(
            "  <updated>{}</updated>\n",
            escape_xml(&self.updated)
        ));
        for link in &self.links {
            let title_attr = link
                .title
                .as_deref()
                .map(|t| format!(" title=\"{}\"", escape_xml(t)))
                .unwrap_or_default();
            out.push_str(&format!(
                "  <link rel=\"{}\" type=\"{}\" href=\"{}\"{}/>\n",
                escape_xml(&link.rel),
                escape_xml(&link.link_type),
                escape_xml(&link.href),
                title_attr,
            ));
        }
        for entry in &self.entries {
            out.push_str("  <entry>\n");
            out.push_str(&format!("    <id>{}</id>\n", escape_xml(&entry.id)));
            out.push_str(&format!(
                "    <title>{}</title>\n",
                escape_xml(&entry.title)
            ));
            out.push_str(&format!(
                "    <updated>{}</updated>\n",
                escape_xml(&entry.updated)
            ));
            if let Some(summary) = &entry.summary {
                out.push_str(&format!("    <summary>{}</summary>\n", escape_xml(summary)));
            }
            for link in &entry.links {
                let title_attr = link
                    .title
                    .as_deref()
                    .map(|t| format!(" title=\"{}\"", escape_xml(t)))
                    .unwrap_or_default();
                out.push_str(&format!(
                    "    <link rel=\"{}\" type=\"{}\" href=\"{}\"{}/>\n",
                    escape_xml(&link.rel),
                    escape_xml(&link.link_type),
                    escape_xml(&link.href),
                    title_attr,
                ));
            }
            out.push_str("  </entry>\n");
        }
        out.push_str("</feed>");
        out
    }
}

pub fn open_search_description() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
     <OpenSearchDescription xmlns=\"http://a9.com/-/spec/opensearch/1.1/\">\
       <ShortName>Harmonia</ShortName>\
       <Description>Search the Harmonia media library</Description>\
       <Url type=\"application/atom+xml;profile=opds-catalog\" \
            template=\"/opds/v1/search.xml?q={searchTerms}\"/>\
       <Url type=\"application/opds+json\" \
            template=\"/opds/v2/search?q={searchTerms}\"/>\
     </OpenSearchDescription>"
        .to_string()
}
