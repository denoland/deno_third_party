"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../manipulation");
var utils_1 = require("../../utils");
var expression_1 = require("../expression");
var QuoteKind_1 = require("./QuoteKind");
exports.StringLiteralBase = expression_1.LiteralExpression;
var StringLiteral = /** @class */ (function (_super) {
    tslib_1.__extends(StringLiteral, _super);
    function StringLiteral() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the literal value.
     *
     * This is equivalent to .getLiteralText() for string literals and only exists for consistency with other literals.
     */
    StringLiteral.prototype.getLiteralValue = function () {
        return this.compilerNode.text;
    };
    /**
     * Sets the literal value.
     * @param value - Value to set.
     */
    StringLiteral.prototype.setLiteralValue = function (value) {
        manipulation_1.replaceNodeText({
            sourceFile: this.sourceFile,
            start: this.getStart() + 1,
            replacingLength: this.getWidth() - 2,
            newText: utils_1.StringUtils.escapeForWithinString(value, this.getQuoteKind())
        });
        return this;
    };
    /**
     * Gets the quote kind.
     */
    StringLiteral.prototype.getQuoteKind = function () {
        return this.getText()[0] === "'" ? QuoteKind_1.QuoteKind.Single : QuoteKind_1.QuoteKind.Double;
    };
    return StringLiteral;
}(exports.StringLiteralBase));
exports.StringLiteral = StringLiteral;
