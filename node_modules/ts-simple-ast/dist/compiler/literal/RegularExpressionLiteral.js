"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../manipulation");
var expression_1 = require("../expression");
exports.RegularExpressionLiteralBase = expression_1.LiteralExpression;
var RegularExpressionLiteral = /** @class */ (function (_super) {
    tslib_1.__extends(RegularExpressionLiteral, _super);
    function RegularExpressionLiteral() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the literal value.
     */
    RegularExpressionLiteral.prototype.getLiteralValue = function () {
        var pattern = /^\/(.*)\/([^\/]*)$/;
        var text = this.compilerNode.text;
        var matches = pattern.exec(text);
        return new RegExp(matches[1], matches[2]);
    };
    RegularExpressionLiteral.prototype.setLiteralValue = function (regExpOrPattern, flags) {
        var pattern;
        if (typeof regExpOrPattern === "string")
            pattern = regExpOrPattern;
        else {
            pattern = regExpOrPattern.source;
            flags = regExpOrPattern.flags;
        }
        manipulation_1.replaceNodeText({
            sourceFile: this.sourceFile,
            start: this.getStart(),
            replacingLength: this.getWidth(),
            newText: "/" + pattern + "/" + (flags || "")
        });
        return this;
    };
    return RegularExpressionLiteral;
}(exports.RegularExpressionLiteralBase));
exports.RegularExpressionLiteral = RegularExpressionLiteral;
