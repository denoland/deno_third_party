"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../../manipulation");
var expression_1 = require("../../expression");
exports.NoSubstitutionTemplateLiteralBase = expression_1.LiteralExpression;
var NoSubstitutionTemplateLiteral = /** @class */ (function (_super) {
    tslib_1.__extends(NoSubstitutionTemplateLiteral, _super);
    function NoSubstitutionTemplateLiteral() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the literal value.
     */
    NoSubstitutionTemplateLiteral.prototype.getLiteralValue = function () {
        // for consistency with other literals
        return this.compilerNode.text;
    };
    /**
     * Sets the literal value.
     *
     * Note: This could possibly replace the node if you add a tagged template.
     * @param value - Value to set.
     * @returns The new node if the kind changed; the current node otherwise.
     */
    NoSubstitutionTemplateLiteral.prototype.setLiteralValue = function (value) {
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
    return NoSubstitutionTemplateLiteral;
}(exports.NoSubstitutionTemplateLiteralBase));
exports.NoSubstitutionTemplateLiteral = NoSubstitutionTemplateLiteral;
