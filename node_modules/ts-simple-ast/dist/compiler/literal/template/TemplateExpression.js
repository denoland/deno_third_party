"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../../manipulation");
var expression_1 = require("../../expression");
exports.TemplateExpressionBase = expression_1.PrimaryExpression;
var TemplateExpression = /** @class */ (function (_super) {
    tslib_1.__extends(TemplateExpression, _super);
    function TemplateExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the template head.
     */
    TemplateExpression.prototype.getHead = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.head);
    };
    /**
     * Gets the template spans.
     */
    TemplateExpression.prototype.getTemplateSpans = function () {
        var _this = this;
        return this.compilerNode.templateSpans.map(function (s) { return _this.getNodeFromCompilerNode(s); });
    };
    /**
     * Sets the literal value.
     *
     * Note: This could possibly replace the node if you remove all the tagged templates.
     * @param value - Value to set.
     * @returns The new node if the kind changed; the current node otherwise.
     */
    TemplateExpression.prototype.setLiteralValue = function (value) {
        var childIndex = this.getChildIndex();
        var parent = this.getParentSyntaxList() || this.getParentOrThrow();
        manipulation_1.replaceNodeText({
            sourceFile: this.sourceFile,
            start: this.getStart() + 1,
            replacingLength: this.getWidth() - 2,
            newText: value
        });
        return parent.getChildAtIndex(childIndex);
    };
    return TemplateExpression;
}(exports.TemplateExpressionBase));
exports.TemplateExpression = TemplateExpression;
