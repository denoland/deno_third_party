"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
function LiteralLikeNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.getLiteralText = function () {
            return this.compilerNode.text;
        };
        class_1.prototype.isTerminated = function () {
            // I'm sorry, but this should not be a negative
            return !(this.compilerNode.isUnterminated || false);
        };
        class_1.prototype.hasExtendedUnicodeEscape = function () {
            return this.compilerNode.hasExtendedUnicodeEscape || false;
        };
        return class_1;
    }(Base));
}
exports.LiteralLikeNode = LiteralLikeNode;
