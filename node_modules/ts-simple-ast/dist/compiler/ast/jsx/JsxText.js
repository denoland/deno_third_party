"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var common_1 = require("../common");
var JsxText = /** @class */ (function (_super) {
    tslib_1.__extends(JsxText, _super);
    function JsxText() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets if the JSX text contains only white spaces.
     */
    JsxText.prototype.containsOnlyWhiteSpaces = function () {
        return this.compilerNode.containsOnlyWhiteSpaces;
    };
    return JsxText;
}(common_1.Node));
exports.JsxText = JsxText;
