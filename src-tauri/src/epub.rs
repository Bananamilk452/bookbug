use crate::utils::Error;
use roxmltree::Document;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, Mutex};
use zip::ZipArchive;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub uuid: String,
    pub isbn: String,
    pub title: String,
    pub creator: String,
    pub language: String,
    pub date_modification: String,
    pub subject: String,
    pub description: String,
    pub rights: String,
    pub relation: String,
    pub date_publication: String,
    pub format: String,
    pub publisher: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EpubManifest {
    pub metadata: Metadata,
    pub spine: Vec<SpineItem>,
    pub manifest: HashMap<String, ManifestItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestItem {
    pub href: String,
    pub media_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpineItem {
    pub idref: String,
}

pub struct EpubStore {
    pub files: HashMap<String, Vec<u8>>,
    pub manifest: EpubManifest,
}

impl EpubStore {
    pub fn load(epub_path: &str) -> Result<Self, Error> {
        let file = std::fs::File::open(epub_path)?;
        let mut archive = ZipArchive::new(file)?;
        let mut files = HashMap::new();

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            if entry.is_file() {
                let name = entry.name().to_string();
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                files.insert(name, buf);
            }
        }

        let manifest = parse_epub(&files)?;

        Ok(Self { files, manifest })
    }
}

pub type EpubState = Arc<Mutex<Option<EpubStore>>>;

fn get_full_path_from_container_xml(contents: &str) -> Result<String, Error> {
    let doc = Document::parse(contents)?;
    let node = doc.descendants().find(|n| n.has_tag_name("rootfile"));
    if let Some(node) = node {
        if let Some(full_path) = node.attribute("full-path") {
            Ok(full_path.to_string())
        } else {
            Err(Error::Epub(
                "'full-path' attribute not found in 'rootfile' element".to_string(),
            ))
        }
    } else {
        Err(Error::Epub(
            "'rootfile' element not found in container.xml".to_string(),
        ))
    }
}

fn extract_metadata_from_opf(doc: &Document) -> Result<Metadata, Error> {
    let metadata_node = doc
        .descendants()
        .find(|n| n.has_tag_name("metadata"))
        .ok_or(Error::Epub(
            "'metadata' element not found in OPF file".to_string(),
        ))?;

    let get_text = |tag: &str| {
        metadata_node
            .descendants()
            .find(|n| n.has_tag_name(tag))
            .and_then(|n| n.text())
            .unwrap_or("")
            .to_string()
    };

    let uuid = metadata_node
        .descendants()
        .filter(|n| n.has_tag_name("identifier"))
        .find(|n| {
            n.attributes()
                .any(|a| a.name() == "scheme" && a.value() == "UUID")
        })
        .and_then(|n| n.text())
        .unwrap_or("")
        .to_string();

    let isbn = metadata_node
        .descendants()
        .filter(|n| n.has_tag_name("identifier"))
        .find(|n| {
            n.attributes()
                .any(|a| a.name() == "scheme" && a.value() == "ISBN")
        })
        .and_then(|n| n.text())
        .unwrap_or("")
        .to_string();

    let date_modification = metadata_node
        .descendants()
        .filter(|n| n.has_tag_name("date"))
        .find(|n| {
            n.attributes()
                .any(|a| a.name() == "event" && a.value() == "modification")
        })
        .and_then(|n| n.text())
        .unwrap_or("")
        .to_string();

    let date_publication = metadata_node
        .descendants()
        .filter(|n| n.has_tag_name("date"))
        .find(|n| {
            n.attributes()
                .any(|a| a.name() == "event" && a.value() == "publication")
        })
        .and_then(|n| n.text())
        .unwrap_or("")
        .to_string();

    Ok(Metadata {
        uuid,
        isbn,
        title: get_text("title"),
        creator: get_text("creator"),
        language: get_text("language"),
        date_modification,
        subject: get_text("subject"),
        description: get_text("description"),
        rights: get_text("rights"),
        relation: get_text("relation"),
        date_publication,
        format: get_text("format"),
        publisher: get_text("publisher"),
    })
}

fn extract_manifest_from_opf(doc: &Document) -> Result<HashMap<String, ManifestItem>, Error> {
    let manifest_node = doc
        .descendants()
        .find(|n| n.has_tag_name("manifest"))
        .ok_or(Error::Epub(
            "'manifest' element not found in OPF file".to_string(),
        ))?;

    let mut manifest = HashMap::new();
    for item in manifest_node.children().filter(|n| n.has_tag_name("item")) {
        if let (Some(id), Some(href), Some(media_type)) = (
            item.attribute("id"),
            item.attribute("href"),
            item.attribute("media-type"),
        ) {
            manifest.insert(
                id.to_string(),
                ManifestItem {
                    href: href.to_string(),
                    media_type: media_type.to_string(),
                },
            );
        }
    }
    Ok(manifest)
}

fn extract_spine_from_opf(doc: &Document) -> Result<Vec<SpineItem>, Error> {
    let spine_node = doc
        .descendants()
        .find(|n| n.has_tag_name("spine"))
        .ok_or(Error::Epub(
            "'spine' element not found in OPF file".to_string(),
        ))?;

    let mut spine = Vec::new();
    for itemref in spine_node.children().filter(|n| n.has_tag_name("itemref")) {
        if let Some(idref) = itemref.attribute("idref") {
            spine.push(SpineItem {
                idref: idref.to_string(),
            });
        }
    }
    Ok(spine)
}

pub fn parse_epub(files: &HashMap<String, Vec<u8>>) -> Result<EpubManifest, Error> {
    let container = files.get("META-INF/container.xml").ok_or(Error::Epub(
        "'META-INF/container.xml' not found in EPUB archive".to_string(),
    ))?;

    let opf_path =
        get_full_path_from_container_xml(std::str::from_utf8(container).map_err(|_| {
            Error::Epub("unable to convert container.xml contents to UTF-8".to_string())
        })?)?;
    let opf = std::str::from_utf8(
        files
            .get(&opf_path)
            .ok_or_else(|| Error::Epub(format!("OPF file not found in archive: {}", opf_path)))?,
    )
    .map_err(|_| Error::Epub("unable to convert OPF contents to UTF-8".to_string()))?;

    let doc = Document::parse(opf)?;
    let metadata = extract_metadata_from_opf(&doc)?;
    let manifest = extract_manifest_from_opf(&doc)?;
    let spine = extract_spine_from_opf(&doc)?;
    let epub_manifest = EpubManifest {
        metadata,
        manifest,
        spine,
    };

    Ok(epub_manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTAINER_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;

    const OPF_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="bookid" version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:opf="http://www.idpf.org/2007/opf">
    <dc:identifier id="bookid" opf:scheme="UUID">urn:uuid:5537d26c-b3d7-4591-981b-c801a1762dbf</dc:identifier>
    <dc:identifier opf:scheme="ISBN">978-89-374-9220-4</dc:identifier>
    <dc:title>테스트 EPUB</dc:title>
    <dc:creator>홍길동</dc:creator>
    <dc:language>ko</dc:language>
    <dc:date opf:event="modification">2024-07-24</dc:date>
    <dc:date opf:event="publication">2024-07-24</dc:date>
    <dc:subject>소설</dc:subject>
    <dc:description>테스트용 EPUB 파일입니다</dc:description>
    <dc:publisher>테스트출판사</dc:publisher>
    <dc:rights>All rights reserved</dc:rights>
    <dc:relation>시리즈 1권</dc:relation>
    <dc:format>EPUB 3.0</dc:format>
  </metadata>
  <manifest>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
    <item id="cover" href="cover.xhtml" media-type="application/xhtml+xml"/>
    <item id="chapter1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
    <item id="css" href="style.css" media-type="text/css"/>
  </manifest>
  <spine>
    <itemref idref="cover"/>
    <itemref idref="chapter1"/>
  </spine>
</package>"#;

    const BAD_CONTAINER_XML: &str = r#"<?xml version="1.0"?>
<container>
  <rootfiles>
    <rootfile/>
  </rootfiles>
</container>"#;

    const CONTAINER_XML_NO_ROOTFILE: &str = r#"<?xml version="1.0"?>
<container>
  <rootfiles>
    <other/>
  </rootfiles>
</container>"#;

    fn make_fake_epub_files(
        opf_path: &str,
        container_xml: &str,
        opf_xml: &str,
    ) -> HashMap<String, Vec<u8>> {
        let mut files = HashMap::new();
        files.insert(
            "META-INF/container.xml".to_string(),
            container_xml.as_bytes().to_vec(),
        );
        files.insert(opf_path.to_string(), opf_xml.as_bytes().to_vec());
        files
    }

    #[test]
    fn test_get_full_path_from_container_xml() {
        let path = get_full_path_from_container_xml(CONTAINER_XML).unwrap();
        assert_eq!(path, "OEBPS/content.opf");
    }

    #[test]
    fn test_get_full_path_missing_attribute() {
        let result = get_full_path_from_container_xml(BAD_CONTAINER_XML);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("'full-path' attribute not found"));
    }

    #[test]
    fn test_get_full_path_missing_rootfile() {
        let result = get_full_path_from_container_xml(CONTAINER_XML_NO_ROOTFILE);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("'rootfile' element not found"));
    }

    #[test]
    fn test_extract_metadata_from_opf() {
        let doc = Document::parse(OPF_XML).unwrap();
        let metadata = extract_metadata_from_opf(&doc).unwrap();
        assert_eq!(
            metadata.uuid,
            "urn:uuid:5537d26c-b3d7-4591-981b-c801a1762dbf"
        );
        assert_eq!(metadata.isbn, "978-89-374-9220-4");
        assert_eq!(metadata.title, "테스트 EPUB");
        assert_eq!(metadata.creator, "홍길동");
        assert_eq!(metadata.language, "ko");
        assert_eq!(metadata.date_modification, "2024-07-24");
        assert_eq!(metadata.date_publication, "2024-07-24");
        assert_eq!(metadata.subject, "소설");
        assert_eq!(metadata.description, "테스트용 EPUB 파일입니다");
        assert_eq!(metadata.publisher, "테스트출판사");
        assert_eq!(metadata.rights, "All rights reserved");
        assert_eq!(metadata.relation, "시리즈 1권");
        assert_eq!(metadata.format, "EPUB 3.0");
    }

    #[test]
    fn test_extract_metadata_missing_fields() {
        let opf = r#"<?xml version="1.0"?>
<package xmlns="http://www.idpf.org/2007/opf">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Minimal</dc:title>
  </metadata>
</package>"#;
        let doc = Document::parse(opf).unwrap();
        let metadata = extract_metadata_from_opf(&doc).unwrap();
        assert_eq!(metadata.title, "Minimal");
        assert_eq!(metadata.creator, "");
        assert_eq!(metadata.language, "");
        assert_eq!(metadata.publisher, "");
    }

    #[test]
    fn test_extract_metadata_no_metadata_element() {
        let opf = r#"<?xml version="1.0"?>
<package>
</package>"#;
        let doc = Document::parse(opf).unwrap();
        let result = extract_metadata_from_opf(&doc);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("metadata' element not found"));
    }

    #[test]
    fn test_extract_manifest_from_opf() {
        let doc = Document::parse(OPF_XML).unwrap();
        let manifest = extract_manifest_from_opf(&doc).unwrap();
        assert_eq!(manifest.len(), 4);
        assert_eq!(manifest.get("ncx").unwrap().href, "toc.ncx");
        assert_eq!(
            manifest.get("ncx").unwrap().media_type,
            "application/x-dtbncx+xml"
        );
        assert_eq!(manifest.get("cover").unwrap().href, "cover.xhtml");
        assert_eq!(
            manifest.get("cover").unwrap().media_type,
            "application/xhtml+xml"
        );
        assert_eq!(manifest.get("chapter1").unwrap().href, "chapter1.xhtml");
        assert_eq!(
            manifest.get("chapter1").unwrap().media_type,
            "application/xhtml+xml"
        );
        assert_eq!(manifest.get("css").unwrap().href, "style.css");
        assert_eq!(manifest.get("css").unwrap().media_type, "text/css");
    }

    #[test]
    fn test_extract_manifest_empty() {
        let opf = r#"<?xml version="1.0"?>
<package>
  <manifest>
  </manifest>
</package>"#;
        let doc = Document::parse(opf).unwrap();
        let manifest = extract_manifest_from_opf(&doc).unwrap();
        assert!(manifest.is_empty());
    }

    #[test]
    fn test_extract_manifest_missing_element() {
        let opf = r#"<?xml version="1.0"?>
<package>
</package>"#;
        let doc = Document::parse(opf).unwrap();
        let result = extract_manifest_from_opf(&doc);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("manifest' element not found"));
    }

    #[test]
    fn test_extract_spine_from_opf() {
        let doc = Document::parse(OPF_XML).unwrap();
        let spine = extract_spine_from_opf(&doc).unwrap();
        assert_eq!(spine.len(), 2);
        assert_eq!(spine[0].idref, "cover");
        assert_eq!(spine[1].idref, "chapter1");
    }

    #[test]
    fn test_extract_spine_empty() {
        let opf = r#"<?xml version="1.0"?>
<package>
  <spine>
  </spine>
</package>"#;
        let doc = Document::parse(opf).unwrap();
        let spine = extract_spine_from_opf(&doc).unwrap();
        assert!(spine.is_empty());
    }

    #[test]
    fn test_extract_spine_missing_element() {
        let opf = r#"<?xml version="1.0"?>
<package>
</package>"#;
        let doc = Document::parse(opf).unwrap();
        let result = extract_spine_from_opf(&doc);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("spine' element not found"));
    }

    #[test]
    fn test_parse_epub_success() {
        let files = make_fake_epub_files("OEBPS/content.opf", CONTAINER_XML, OPF_XML);
        let epub = parse_epub(&files).unwrap();
        assert_eq!(epub.metadata.title, "테스트 EPUB");
        assert_eq!(epub.spine.len(), 2);
        assert_eq!(epub.manifest.len(), 4);
    }

    #[test]
    fn test_parse_epub_opf_path_not_found_in_files() {
        let files = make_fake_epub_files("OEBPS/missing.opf", CONTAINER_XML, OPF_XML);
        let result = parse_epub(&files);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("OPF file not found in archive"));
    }

    #[test]
    fn test_parse_epub_broken_container_xml() {
        let mut files = HashMap::new();
        files.insert(
            "META-INF/container.xml".to_string(),
            b"not valid xml".to_vec(),
        );
        let result = parse_epub(&files);
        assert!(result.is_err());
    }

    #[test]
    fn test_epub_store_load_real_file() {
        let manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let epub_path = manifest_path.join("./tests/test.epub");

        let store = EpubStore::load(epub_path.to_str().unwrap()).unwrap();

        assert!(!store.files.is_empty());
        assert!(store.files.contains_key("META-INF/container.xml"));

        assert!(!store.manifest.metadata.title.is_empty());
        assert!(!store.manifest.spine.is_empty());
        assert!(!store.manifest.manifest.is_empty());
    }
}
