"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../manipulation");
var expression_1 = require("../expression");
exports.NumericLiteralBase = expression_1.LiteralExpression;
var NumericLiteral = /** @class */ (function (_super) {
    tslib_1.__extends(NumericLiteral, _super);
    function NumericLiteral() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the literal value.
     */
    NumericLiteral.prototype.getLiteralValue = function () {
        var text = this.compilerNode.text;
        if (text.indexOf(".") >= 0)
            return parseFloat(text);
        return parseInt(text, 10);
    };
    /**
     * Sets the literal value.
     * @param value - Value to set.
     */
    NumericLiteral.prototype.setLiteralValue = function (value) {
        manipulation_1.replaceNodeText({
            sourceFile: this.sourceFile,
            start: this.getStart(),
            replacingLength: this.getWidth(),
            newText: value.toString(10)
        });
        return this;
    };
    return NumericLiteral;
}(exports.NumericLiteralBase));
exports.NumericLiteral = NumericLiteral;
