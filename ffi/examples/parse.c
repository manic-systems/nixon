#include <stdio.h>
#include <string.h>

#include "nixon.h"

int main(void) {
  const char *source = "{ answer = missing; }";
  NixonDocument *document = NULL;
  NixonStatus status =
      nixon_parse((const uint8_t *)source, strlen(source), &document);
  if (status != NIXON_STATUS_OK) {
    fprintf(stderr, "nixon_parse failed with status %u\n", status);
    return 1;
  }

  printf("%zu syntax elements, valid=%u\n",
         nixon_document_element_count(document),
         nixon_document_is_valid(document));
  for (size_t i = 0; i < nixon_document_diagnostic_count(document); ++i) {
    NixonDiagnostic diagnostic;
    if (nixon_document_diagnostic(document, i, &diagnostic)) {
      printf("%u:%u: %.*s\n", diagnostic.start, diagnostic.end,
             (int)diagnostic.message.len,
             (const char *)diagnostic.message.data);
    }
  }

  nixon_document_free(document);
  return 0;
}
