extern crate proc_macro;

mod parser;

use std::fmt::format;

use js_sys::JSON::stringify;
use markdown::token;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::quote;
use syn::__private::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::token::Brace;
use syn::{braced, parse_macro_input, Block, Expr, Ident, Result, Token};

// use crate::fmt::Debug;

struct Tokenizer;

impl Tokenizer {
    fn new() -> Self {
        Tokenizer
    }

    // TODO: `<br />` に対応する
    fn open(self, input: &mut ParseStream, open: Span) -> Result<Token> {
        let name = input.parse::<Ident>()?;

        // class="foo" id="app" のような attribute を parse する
        let attributes = self.parse_attributes(input)?;

        // > に該当するものを parse する
        let close = input.parse::<Token![>]>()?;
        let close = close.span();

        Ok(Token::Open {
            name,
            attributes,
            open,
            close,
        })
    }

    fn parse_attributes(self, input: &mut ParseStream) -> Result<Vec<Attribute>> {
        let mut attributes = Vec::new();

        while input.peek(Ident) {
            // class="foo" のような attribute を parse する
            let key = input.parse::<Ident>()?;
            input.parse::<Token![=]>()?;

            let mut value_tokens = proc_macro2::TokenStream::new();
            loop {
                let token_tree = input.parse::<TokenTree>()?;
                value_tokens.extend(Some(token_tree));

                // > があれば終了(/> で終了する場合)
                // / があれば終了(/> で終了する場合)
                // = があれば終了(次の attribute を読みにいく)
                if input.peek(Token![>]) || input.peek(Token![/]) || input.peek2(Token![=]) {
                    break;
                }
            }

            let value = syn::parse2::<Expr>(value_tokens)?;

            attributes.push(Attribute {
                key: key.clone().into_token_stream(),
                value,
                span: key.span(),
            });
        }

        Ok(attributes)
    }

    // </div> みたいなやつ
    fn close(self, input: &mut ParseStream, close: Span) -> Result<Token> {
        // /div> に該当するものを parse する
        input.parse::<Token![/]>()?;

        let name = input.parse::<Ident>()?;

        // > に該当するものをparseする
        input.parse::<Token![>]>()?;

        Ok(Token::Close { name, close })
    }

    // なんでもないやつ
    fn text(self, input: &mut ParseStream) -> Result<Token> {
        let mut content = "".to_string();
        let mut i = 0;
        let mut start = None;
        let mut end: Option<Span> = None;

        // attirbutes と同じ方式で parse する
        loop {
            if input.is_empty() {
                break;
            }

            let token_tree = input.parse::<TokenTree>()?;

            if i == 0 {
                start = Some(token_tree.span());
                end = Some(token_tree.span());
            } else {
                if let Some(end) = end {}
            }

            content += &token_tree.to_string();

            end = Some(token_tree.span());

            // string literal<div> のように開始タグがあれば string は終了
            // 同じく string literal{variable} のように変数があれば string は終了
            if input.peek(Token![<]) || input.peek(Brace) {
                break;
            }

            i += 1;
        }

        Ok(Token::Text {
            content,
            start,
            end,
        })
    }

    // {variable} 的な
    fn braced(self, input: &mut ParseStream) -> Result<Token> {
        let content;
        let brace_token = braced!(content in input);

        // TODO: content.call(syn::Block::parse_within) で複数の式を parse できるようにする
        // let blocks = content.call(Block::parse_within)?;
        let block = content.parse::<Expr>()?;

        Ok(Token::Braced {
            block,
            span: brace_token.span.span(),
        })
    }
}

#[derive(Debug)]
struct Attribute {
    key: proc_macro2::TokenStream,
    value: Expr,
    span: Span,
}

impl Attribute {
    fn key_string(&self) -> String {
        self.key.to_string()
    }

    fn value_string(&self) -> String {
        self.value.to_token_stream().to_string()
    }
}

#[derive(Debug)]
enum Token {
    Open {
        name: Ident,
        open: Span,
        close: Span,
        attributes: Vec<Attribute>,
    },
    Close {
        name: Ident,
        close: Span,
    },
    Text {
        content: String,
        start: Option<Span>,
        end: Option<Span>,
    },
    // 今のところこれだけあれば十分な気がする
    Braced {
        block: Expr,
        span: Span,
    },
}

#[derive(Debug)]
struct Render {
    tokens: Vec<Token>,
}

impl Parse for Render {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut tokens = Vec::new();

        while !input.is_empty() {
            let token = input.parse::<Token>()?;
            tokens.push(token);
        }

        Ok(Render { tokens })
    }
}

impl Parse for Token {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut input = input;
        let tokenizer = Tokenizer::new();

        if input.peek(Token![<]) {
            let open = input.parse::<Token![<]>()?;
            let span = open.span();

            if input.peek(Token![/]) {
                // < の次に / があれば閉じタグ(e.g. </div>)
                return tokenizer.close(&mut input, span);
            } else {
                // それ以外は開きタグ(e.g. <div>)
                return tokenizer.open(&mut input, span);
            }
        }

        // {} で式を埋め込むことができるようにする
        if input.peek(Brace) {
            return tokenizer.braced(&mut input);
        }

        return tokenizer.text(&mut input);
    }
}

/// let foo = "hello world";
/// render! { <div class="foo" id="app">{foo}</div> } -> "<div class="foo" id="app">hello world</div>
#[proc_macro]
pub fn render(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed_input = parse_macro_input!(input as Render);
    let tokens = parsed_input.tokens;

    let mut parser = parser::Parser::new();
    let tokens = parser.create_node(tokens);

    tokens.into()
}
