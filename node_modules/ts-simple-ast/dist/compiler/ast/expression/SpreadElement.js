"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var Expression_1 = require("./Expression");
var expressioned_1 = require("./expressioned");
exports.SpreadElementBase = expressioned_1.ExpressionedNode(Expression_1.Expression);
var SpreadElement = /** @class */ (function (_super) {
    tslib_1.__extends(SpreadElement, _super);
    function SpreadElement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return SpreadElement;
}(exports.SpreadElementBase));
exports.SpreadElement = SpreadElement;
