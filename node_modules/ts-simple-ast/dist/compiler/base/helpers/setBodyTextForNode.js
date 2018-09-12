"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var manipulation_1 = require("../../../manipulation");
var typescript_1 = require("../../../typescript");
var getBodyText_1 = require("./getBodyText");
/**
 * @internal
 */
function setBodyTextForNode(body, textOrWriterFunction) {
    var newText = getBodyText_1.getBodyText(body.getWriterWithIndentation(), textOrWriterFunction);
    var openBrace = body.getFirstChildByKindOrThrow(typescript_1.SyntaxKind.OpenBraceToken);
    var closeBrace = body.getFirstChildByKindOrThrow(typescript_1.SyntaxKind.CloseBraceToken);
    manipulation_1.insertIntoParentTextRange({
        insertPos: openBrace.getEnd(),
        newText: newText,
        parent: body,
        replacing: {
            textLength: closeBrace.getStart() - openBrace.getEnd()
        }
    });
}
exports.setBodyTextForNode = setBodyTextForNode;
