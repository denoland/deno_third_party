"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var utils_1 = require("../../../utils");
/**
 * @internal
 */
function getBodyText(writer, textOrWriterFunction) {
    writer.newLineIfLastNot();
    if (typeof textOrWriterFunction !== "string" || textOrWriterFunction.length > 0)
        writer.indentBlock(function () {
            utils_1.printTextFromStringOrWriter(writer, textOrWriterFunction);
        });
    writer.newLineIfLastNot();
    writer.write(""); // write last line's indentation
    return writer.toString();
}
exports.getBodyText = getBodyText;
