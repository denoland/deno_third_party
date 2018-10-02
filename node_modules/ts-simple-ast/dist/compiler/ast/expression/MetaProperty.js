"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../base");
var PrimaryExpression_1 = require("./PrimaryExpression");
exports.MetaPropertyBase = base_1.NamedNode(PrimaryExpression_1.PrimaryExpression);
var MetaProperty = /** @class */ (function (_super) {
    tslib_1.__extends(MetaProperty, _super);
    function MetaProperty() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the keyword token.
     */
    MetaProperty.prototype.getKeywordToken = function () {
        return this.compilerNode.keywordToken;
    };
    return MetaProperty;
}(exports.MetaPropertyBase));
exports.MetaProperty = MetaProperty;
