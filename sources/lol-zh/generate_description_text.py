#!/usr/bin/env python3
"""Offline source-local description projection; read retained original JSON, no network.

The guest validates raw metadata against frozen_contract before emitting this table.
Pure text is unchanged. HTML paragraph nodes, including empty paragraphs, become
text paragraphs; br becomes one LF. HTMLParser resolves HTML5 named/numeric entities
as characters while encoded angle brackets remain literal text, never reparsed.
"""
from html.parser import HTMLParser
from pathlib import Path
import argparse,json,re

class ParagraphText(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.paragraphs=[]
        self.current=[]
        self.inside=False
    def handle_starttag(self,tag,attrs):
        if tag=='p':
            if self.inside:
                raise ValueError('Nested paragraph not in the reviewed contract')
            if self.current:
                text=''.join(self.current)
                if text.strip():self.paragraphs.append(text)
                self.current=[]
            self.inside=True
        elif tag=='br':self.current.append('\n')
    def handle_endtag(self,tag):
        if tag=='p':
            if not self.inside:raise ValueError('Unpaired paragraph')
            self.paragraphs.append(''.join(self.current))
            self.current=[]
            self.inside=False
    def handle_data(self,data):
        if self.inside or not data.isspace():self.current.append(data)
    def text(self):
        if self.inside:raise ValueError('Unclosed paragraph')
        if self.current:self.paragraphs.append(''.join(self.current))
        return '\n\n'.join(self.paragraphs)

def plain_text(raw):
    if not re.search(r'</?[A-Za-z][^>]*>',raw):return raw
    parser=ParagraphText();parser.feed(raw);parser.close();return parser.text()

def generate(original_root,metadata_root):
    catalog=json.loads((original_root/'catalog-localized.json').read_text())
    rows=[]
    for card in catalog['sections']['series']['data']:
        slug=card['url'].split('/')[-1]
        path=original_root/'work-lux.json' if slug=='lux' else metadata_root/(slug+'-metadata.json')
        metadata=json.loads(path.read_text())
        assert metadata['id']==slug and metadata['locale']=='zh_cn' and metadata['name']==card['title']
        raw=metadata['description'];text=plain_text(raw)
        rows.append({'slug':slug,'raw':raw,'text':text,'changed':raw!=text,'metadataFile':str(path)})
    lines=['// Offline projection of exact retained original metadata; raw guest validation is unchanged.',
           'use super::{Book, R};','pub(super) fn for_book(book: &Book) -> R<&\'static str> {','    match book.slug {']
    for row in rows:lines.append('        '+json.dumps(row['slug'])+' => Ok('+json.dumps(row['text'],ensure_ascii=False)+'),')
    lines.extend(['        _ => Err("unknown_contract"),','    }','}'])
    return '\n'.join(lines)+'\n',rows

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--original-root',type=Path,required=True);parser.add_argument('--metadata-root',type=Path,required=True);parser.add_argument('--output',type=Path,required=True);parser.add_argument('--receipt',type=Path,required=True);args=parser.parse_args()
    text,rows=generate(args.original_root,args.metadata_root);args.output.write_text(text);args.receipt.write_text(json.dumps(rows,ensure_ascii=False,indent=2)+'\n')
