"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../base");
var expressioned_1 = require("./expressioned");
var MemberExpression_1 = require("./MemberExpression");
exports.PropertyAccessExpressionBase = base_1.NamedNode(expressioned_1.LeftHandSideExpressionedNode(MemberExpression_1.MemberExpression));
var PropertyAccessExpression = /** @class */ (function (_super) {
    tslib_1.__extends(PropertyAccessExpression, _super);
    function PropertyAccessExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return PropertyAccessExpression;
}(exports.PropertyAccessExpressionBase));
exports.PropertyAccessExpression = PropertyAccessExpression;
