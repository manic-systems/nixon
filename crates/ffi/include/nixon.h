#ifndef NIXON_H
#define NIXON_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct NixonDocument NixonDocument;

typedef uint8_t NixonStatus;
#define NIXON_STATUS_OK UINT8_C(0)
#define NIXON_STATUS_INVALID_ARGUMENT UINT8_C(1)
#define NIXON_STATUS_INVALID_UTF8 UINT8_C(2)
#define NIXON_STATUS_TOO_LARGE UINT8_C(3)
#define NIXON_STATUS_TOO_MANY_ELEMENTS UINT8_C(4)
#define NIXON_STATUS_UNKNOWN_INPUT_ERROR UINT8_C(5)

typedef uint8_t NixonSeverity;
#define NIXON_SEVERITY_ERROR UINT8_C(0)
#define NIXON_SEVERITY_WARNING UINT8_C(1)

typedef uint8_t NixonDiagnosticKind;
#define NIXON_DIAGNOSTIC_UNEXPECTED_TOKEN UINT8_C(0)
#define NIXON_DIAGNOSTIC_UNEXPECTED_EOF UINT8_C(1)
#define NIXON_DIAGNOSTIC_INVALID_TOKEN UINT8_C(2)
#define NIXON_DIAGNOSTIC_TRAILING_INPUT UINT8_C(3)
#define NIXON_DIAGNOSTIC_NESTING_LIMIT UINT8_C(4)
#define NIXON_DIAGNOSTIC_EXPERIMENTAL_FEATURE_DISABLED UINT8_C(5)
#define NIXON_DIAGNOSTIC_URI_LITERAL UINT8_C(6)
#define NIXON_DIAGNOSTIC_INTEGER_OVERFLOW UINT8_C(7)
#define NIXON_DIAGNOSTIC_FLOAT_OUT_OF_RANGE UINT8_C(8)
#define NIXON_DIAGNOSTIC_TRAILING_SLASH_PATH UINT8_C(9)
#define NIXON_DIAGNOSTIC_DUPLICATE_FORMAL UINT8_C(10)
#define NIXON_DIAGNOSTIC_DUPLICATE_ATTRIBUTE UINT8_C(11)
#define NIXON_DIAGNOSTIC_CONFLICTING_ATTRIBUTE UINT8_C(12)
#define NIXON_DIAGNOSTIC_DYNAMIC_ATTRIBUTE_IN_LET UINT8_C(13)
#define NIXON_DIAGNOSTIC_DYNAMIC_ATTRIBUTE_IN_INHERIT UINT8_C(14)
#define NIXON_DIAGNOSTIC_UNDEFINED_VARIABLE UINT8_C(15)

typedef uint8_t NixonSyntaxKind;
#define NIXON_SYNTAX_ERROR UINT8_C(0)
#define NIXON_SYNTAX_EOF UINT8_C(1)
#define NIXON_SYNTAX_WHITESPACE UINT8_C(2)
#define NIXON_SYNTAX_LINE_COMMENT UINT8_C(3)
#define NIXON_SYNTAX_BLOCK_COMMENT UINT8_C(4)
#define NIXON_SYNTAX_DOC_COMMENT UINT8_C(5)
#define NIXON_SYNTAX_IDENTIFIER UINT8_C(6)
#define NIXON_SYNTAX_INTEGER UINT8_C(7)
#define NIXON_SYNTAX_FLOAT UINT8_C(8)
#define NIXON_SYNTAX_STRING_FRAGMENT UINT8_C(9)
#define NIXON_SYNTAX_PATH_FRAGMENT UINT8_C(10)
#define NIXON_SYNTAX_PATH UINT8_C(11)
#define NIXON_SYNTAX_SEARCH_PATH UINT8_C(12)
#define NIXON_SYNTAX_URI UINT8_C(13)
#define NIXON_SYNTAX_IF_KEYWORD UINT8_C(14)
#define NIXON_SYNTAX_THEN_KEYWORD UINT8_C(15)
#define NIXON_SYNTAX_ELSE_KEYWORD UINT8_C(16)
#define NIXON_SYNTAX_ASSERT_KEYWORD UINT8_C(17)
#define NIXON_SYNTAX_WITH_KEYWORD UINT8_C(18)
#define NIXON_SYNTAX_LET_KEYWORD UINT8_C(19)
#define NIXON_SYNTAX_IN_KEYWORD UINT8_C(20)
#define NIXON_SYNTAX_REC_KEYWORD UINT8_C(21)
#define NIXON_SYNTAX_INHERIT_KEYWORD UINT8_C(22)
#define NIXON_SYNTAX_OR_KEYWORD UINT8_C(23)
#define NIXON_SYNTAX_LEFT_PAREN UINT8_C(24)
#define NIXON_SYNTAX_RIGHT_PAREN UINT8_C(25)
#define NIXON_SYNTAX_LEFT_BRACE UINT8_C(26)
#define NIXON_SYNTAX_RIGHT_BRACE UINT8_C(27)
#define NIXON_SYNTAX_LEFT_BRACKET UINT8_C(28)
#define NIXON_SYNTAX_RIGHT_BRACKET UINT8_C(29)
#define NIXON_SYNTAX_QUOTE UINT8_C(30)
#define NIXON_SYNTAX_INDENTED_QUOTE UINT8_C(31)
#define NIXON_SYNTAX_INTERPOLATION_START UINT8_C(32)
#define NIXON_SYNTAX_COMMA UINT8_C(33)
#define NIXON_SYNTAX_SEMICOLON UINT8_C(34)
#define NIXON_SYNTAX_COLON UINT8_C(35)
#define NIXON_SYNTAX_AT UINT8_C(36)
#define NIXON_SYNTAX_DOT UINT8_C(37)
#define NIXON_SYNTAX_ELLIPSIS UINT8_C(38)
#define NIXON_SYNTAX_ASSIGN UINT8_C(39)
#define NIXON_SYNTAX_EQUAL UINT8_C(40)
#define NIXON_SYNTAX_NOT_EQUAL UINT8_C(41)
#define NIXON_SYNTAX_LESS UINT8_C(42)
#define NIXON_SYNTAX_GREATER UINT8_C(43)
#define NIXON_SYNTAX_LESS_EQUAL UINT8_C(44)
#define NIXON_SYNTAX_GREATER_EQUAL UINT8_C(45)
#define NIXON_SYNTAX_PLUS UINT8_C(46)
#define NIXON_SYNTAX_MINUS UINT8_C(47)
#define NIXON_SYNTAX_STAR UINT8_C(48)
#define NIXON_SYNTAX_SLASH UINT8_C(49)
#define NIXON_SYNTAX_BANG UINT8_C(50)
#define NIXON_SYNTAX_AND UINT8_C(51)
#define NIXON_SYNTAX_OR UINT8_C(52)
#define NIXON_SYNTAX_IMPLICATION UINT8_C(53)
#define NIXON_SYNTAX_UPDATE UINT8_C(54)
#define NIXON_SYNTAX_CONCATENATE UINT8_C(55)
#define NIXON_SYNTAX_QUESTION UINT8_C(56)
#define NIXON_SYNTAX_PIPE_FROM UINT8_C(57)
#define NIXON_SYNTAX_PIPE_INTO UINT8_C(58)
#define NIXON_SYNTAX_ROOT UINT8_C(59)
#define NIXON_SYNTAX_ERROR_NODE UINT8_C(60)
#define NIXON_SYNTAX_LITERAL UINT8_C(61)
#define NIXON_SYNTAX_STRING UINT8_C(62)
#define NIXON_SYNTAX_INTERPOLATION UINT8_C(63)
#define NIXON_SYNTAX_PATH_EXPRESSION UINT8_C(64)
#define NIXON_SYNTAX_PARENTHESIZED UINT8_C(65)
#define NIXON_SYNTAX_LIST UINT8_C(66)
#define NIXON_SYNTAX_ATTRIBUTE_SET UINT8_C(67)
#define NIXON_SYNTAX_BINDING UINT8_C(68)
#define NIXON_SYNTAX_ATTRIBUTE_PATH UINT8_C(69)
#define NIXON_SYNTAX_INHERIT UINT8_C(70)
#define NIXON_SYNTAX_LET_IN UINT8_C(71)
#define NIXON_SYNTAX_LEGACY_LET UINT8_C(72)
#define NIXON_SYNTAX_IF_THEN_ELSE UINT8_C(73)
#define NIXON_SYNTAX_ASSERT UINT8_C(74)
#define NIXON_SYNTAX_WITH UINT8_C(75)
#define NIXON_SYNTAX_LAMBDA UINT8_C(76)
#define NIXON_SYNTAX_FORMAL_SET UINT8_C(77)
#define NIXON_SYNTAX_FORMAL UINT8_C(78)
#define NIXON_SYNTAX_APPLY UINT8_C(79)
#define NIXON_SYNTAX_SELECT UINT8_C(80)
#define NIXON_SYNTAX_HAS_ATTRIBUTE UINT8_C(81)
#define NIXON_SYNTAX_UNARY_OPERATION UINT8_C(82)
#define NIXON_SYNTAX_BINARY_OPERATION UINT8_C(83)

#define NIXON_ELEMENT_NONE UINT32_MAX

typedef struct NixonString {
  const uint8_t *data;
  size_t len;
} NixonString;

typedef struct NixonElement {
  NixonSyntaxKind kind;
  uint32_t start;
  uint32_t end;
  uint32_t parent;
  uint32_t first_child;
  uint32_t next_sibling;
} NixonElement;

typedef struct NixonDiagnostic {
  NixonDiagnosticKind kind;
  NixonSeverity severity;
  uint32_t start;
  uint32_t end;
  NixonString message;
} NixonDiagnostic;

/* Parses source with the default settings. The source is only borrowed during
 * this call. Syntax errors are returned as diagnostics. On success, the caller
 * owns *document and must free it with nixon_document_free. */
NixonStatus nixon_parse(const uint8_t *source, size_t source_len,
                        NixonDocument **document);

/* Accepts NULL. Every other document pointer must refer to a live handle from
 * nixon_parse. A handle may only be freed once. */
void nixon_document_free(NixonDocument *document);
uint8_t nixon_document_is_valid(const NixonDocument *document);
size_t nixon_document_element_count(const NixonDocument *document);
uint8_t nixon_document_element(const NixonDocument *document, uint32_t id,
                               NixonElement *element);
size_t nixon_document_diagnostic_count(const NixonDocument *document);

/* The returned message bytes remain owned by the document and are not
 * null-terminated. They become invalid when the document is freed. */
uint8_t nixon_document_diagnostic(const NixonDocument *document, size_t id,
                                  NixonDiagnostic *diagnostic);

#ifdef __cplusplus
}
#endif

#endif
