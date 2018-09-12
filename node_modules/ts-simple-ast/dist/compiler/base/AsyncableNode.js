"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var typescript_1 = require("../../typescript");
var callBaseFill_1 = require("../callBaseFill");
function AsyncableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.isAsync = function () {
            return this.hasModifier(typescript_1.SyntaxKind.AsyncKeyword);
        };
        class_1.prototype.getAsyncKeyword = function () {
            return this.getFirstModifierByKind(typescript_1.SyntaxKind.AsyncKeyword);
        };
        class_1.prototype.getAsyncKeywordOrThrow = function () {
            return errors.throwIfNullOrUndefined(this.getAsyncKeyword(), "Expected to find an async keyword.");
        };
        class_1.prototype.setIsAsync = function (value) {
            this.toggleModifier("async", value);
            return this;
        };
        class_1.prototype.fill = function (structure) {
            callBaseFill_1.callBaseFill(Base.prototype, this, structure);
            if (structure.isAsync != null)
                this.setIsAsync(structure.isAsync);
            return this;
        };
        return class_1;
    }(Base));
}
exports.AsyncableNode = AsyncableNode;
