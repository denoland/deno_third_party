"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var common_1 = require("../../common");
var expression_1 = require("../../expression");
exports.TemplateSpanBase = expression_1.ExpressionedNode(common_1.Node);
var TemplateSpan = /** @class */ (function (_super) {
    tslib_1.__extends(TemplateSpan, _super);
    function TemplateSpan() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the template literal.
     */
    TemplateSpan.prototype.getLiteral = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.literal);
    };
    return TemplateSpan;
}(exports.TemplateSpanBase));
exports.TemplateSpan = TemplateSpan;
