"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var manipulation_1 = require("../../manipulation");
var typescript_1 = require("../../typescript");
var utils_1 = require("../../utils");
var base_1 = require("../base");
var TypeNode_1 = require("./TypeNode");
exports.ImportTypeNodeBase = base_1.TypeArgumentedNode(TypeNode_1.TypeNode);
var ImportTypeNode = /** @class */ (function (_super) {
    tslib_1.__extends(ImportTypeNode, _super);
    function ImportTypeNode() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Sets the argument text.
     * @param text - Text of the argument.
     */
    ImportTypeNode.prototype.setArgument = function (text) {
        var arg = this.getArgument();
        if (utils_1.TypeGuards.isLiteralTypeNode(arg)) {
            var literal = arg.getLiteral();
            if (utils_1.TypeGuards.isStringLiteral(literal)) {
                literal.setLiteralValue(text);
                return this;
            }
        }
        arg.replaceWithText(function (writer) { return writer.quote(text); }, this.getWriterWithQueuedChildIndentation());
        return this;
    };
    /**
     * Gets the argument passed into the import type.
     */
    ImportTypeNode.prototype.getArgument = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.argument);
    };
    /**
     * Sets the qualifier text.
     * @param text - Text.
     */
    ImportTypeNode.prototype.setQualifier = function (text) {
        var qualifier = this.getQualifier();
        if (qualifier != null)
            qualifier.replaceWithText(text, this.getWriterWithQueuedChildIndentation());
        else {
            var paren = this.getFirstChildByKindOrThrow(typescript_1.SyntaxKind.CloseParenToken);
            manipulation_1.insertIntoParentTextRange({
                insertPos: paren.getEnd(),
                parent: this,
                newText: this.getWriterWithQueuedIndentation().write(".").write(text).toString()
            });
        }
        return this;
    };
    /**
     * Gets the qualifier of the import type if it exists or throws
     */
    ImportTypeNode.prototype.getQualifierOrThrow = function () {
        var _this = this;
        return errors.throwIfNullOrUndefined(this.getQualifier(), function () { return "Expected to find a qualifier for the import type: " + _this.getText(); });
    };
    /**
     * Gets the qualifier of the import type if it exists or returns undefined.
     */
    ImportTypeNode.prototype.getQualifier = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.qualifier);
    };
    return ImportTypeNode;
}(exports.ImportTypeNodeBase));
exports.ImportTypeNode = ImportTypeNode;
