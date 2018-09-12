"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../base");
var expressioned_1 = require("./expressioned");
var UnaryExpression_1 = require("./UnaryExpression");
exports.TypeAssertionBase = base_1.TypedNode(expressioned_1.UnaryExpressionedNode(UnaryExpression_1.UnaryExpression));
var TypeAssertion = /** @class */ (function (_super) {
    tslib_1.__extends(TypeAssertion, _super);
    function TypeAssertion() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return TypeAssertion;
}(exports.TypeAssertionBase));
exports.TypeAssertion = TypeAssertion;
