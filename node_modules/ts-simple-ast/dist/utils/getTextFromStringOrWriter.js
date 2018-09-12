"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
function getTextFromStringOrWriter(writer, textOrWriterFunction) {
    // note: this should always use a writer to ensure the proper indentation is used
    printTextFromStringOrWriter(writer, textOrWriterFunction);
    return writer.toString();
}
exports.getTextFromStringOrWriter = getTextFromStringOrWriter;
function printTextFromStringOrWriter(writer, textOrWriterFunction) {
    if (typeof textOrWriterFunction === "string")
        writer.write(textOrWriterFunction);
    else
        textOrWriterFunction(writer);
}
exports.printTextFromStringOrWriter = printTextFromStringOrWriter;
