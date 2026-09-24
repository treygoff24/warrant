use oxc_allocator::Allocator;
use oxc_ast::{
    AstKind,
    ast::{Argument, Expression, ImportDeclarationSpecifier},
};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType, Span};

pub(super) struct Request {
    pub start: i64,
    pub end: i64,
    pub specifier: String,
    pub name: Option<String>,
}

pub(super) fn collect(path: &str, bytes: &[u8]) -> Vec<Request> {
    let (Ok(source), Ok(source_type)) = (std::str::from_utf8(bytes), SourceType::from_path(path))
    else {
        return vec![];
    };
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.diagnostics.is_empty() {
        return vec![];
    }
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(allocator.alloc(parsed.program))
        .semantic;
    let mut requests = Vec::new();
    let mut push = |span: Span, specifier: &str, name: Option<String>| {
        requests.push(Request {
            start: i64::from(span.start),
            end: i64::from(span.end),
            specifier: specifier.into(),
            name,
        });
    };
    for node in semantic.nodes().iter() {
        match node.kind() {
            AstKind::ImportDeclaration(declaration) => {
                if let Some(specifiers) = &declaration.specifiers
                    && !specifiers.is_empty()
                {
                    for specifier in specifiers {
                        let name = match specifier {
                            ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                                Some(specifier.imported.name().to_string())
                            }
                            ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                                Some("default".into())
                            }
                            ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {
                                Some("*".into())
                            }
                        };
                        push(specifier.span(), &declaration.source.value, name);
                    }
                } else {
                    push(declaration.span, &declaration.source.value, None);
                }
            }
            AstKind::ExportFromDeclaration(declaration) => {
                for specifier in &declaration.specifiers {
                    push(
                        specifier.span,
                        &declaration.source.value,
                        Some(specifier.local.name().to_string()),
                    );
                }
            }
            AstKind::ExportAllDeclaration(declaration) => push(
                declaration.span,
                &declaration.source.value,
                declaration.exported.as_ref().map(|_| "*".into()),
            ),
            AstKind::ImportExpression(import) => {
                if let Expression::StringLiteral(source) = &import.source {
                    push(import.span, &source.value, None);
                }
            }
            AstKind::CallExpression(call) => {
                if let Expression::Identifier(identifier) = &call.callee
                    && identifier.name == "require"
                    && semantic.is_reference_to_global_variable(identifier)
                    && call.arguments.len() == 1
                    && let Some(Argument::StringLiteral(source)) = call.arguments.first()
                {
                    push(call.span, &source.value, None);
                }
            }
            _ => {}
        }
    }
    requests
}
