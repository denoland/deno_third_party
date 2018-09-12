"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var expression_1 = require("../expression");
var JsxFragment = /** @class */ (function (_super) {
    tslib_1.__extends(JsxFragment, _super);
    function JsxFragment() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the children of the JSX fragment.
     */
    JsxFragment.prototype.getJsxChildren = function () {
        var _this = this;
        return this.compilerNode.children.map(function (c) { return _this.getNodeFromCompilerNode(c); });
    };
    /**
     * Gets the opening fragment.
     */
    JsxFragment.prototype.getOpeningFragment = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.openingFragment);
    };
    /**
     * Gets the closing fragment.
     */
    JsxFragment.prototype.getClosingFragment = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.closingFragment);
    };
    return JsxFragment;
}(expression_1.PrimaryExpression));
exports.JsxFragment = JsxFragment;
