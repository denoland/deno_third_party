"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../manipulation");
var utils_1 = require("../../utils");
var helpers_1 = require("../base/helpers");
var expression_1 = require("../expression");
var JsxElement = /** @class */ (function (_super) {
    tslib_1.__extends(JsxElement, _super);
    function JsxElement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the children of the JSX element.
     */
    JsxElement.prototype.getJsxChildren = function () {
        var _this = this;
        return this.compilerNode.children.map(function (c) { return _this.getNodeFromCompilerNode(c); });
    };
    /**
     * Gets the opening element.
     */
    JsxElement.prototype.getOpeningElement = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.openingElement);
    };
    /**
     * Gets the closing element.
     */
    JsxElement.prototype.getClosingElement = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.closingElement);
    };
    JsxElement.prototype.setBodyText = function (textOrWriterFunction) {
        var newText = helpers_1.getBodyText(this.getWriterWithIndentation(), textOrWriterFunction);
        setText(this, newText);
        return this;
    };
    JsxElement.prototype.setBodyTextInline = function (textOrWriterFunction) {
        var writer = this.getWriterWithQueuedChildIndentation();
        utils_1.printTextFromStringOrWriter(writer, textOrWriterFunction);
        if (writer.isLastNewLine()) {
            writer.setIndentationLevel(Math.max(0, this.getIndentationLevel() - 1));
            writer.write(""); // indentation
        }
        setText(this, writer.toString());
        return this;
    };
    return JsxElement;
}(expression_1.PrimaryExpression));
exports.JsxElement = JsxElement;
function setText(element, newText) {
    var openingElement = element.getOpeningElement();
    var closingElement = element.getClosingElement();
    manipulation_1.insertIntoParentTextRange({
        insertPos: openingElement.getEnd(),
        newText: newText,
        parent: element.getChildSyntaxListOrThrow(),
        replacing: {
            textLength: closingElement.getStart() - openingElement.getEnd()
        }
    });
}
