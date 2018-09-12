"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var typescript_1 = require("../../typescript");
var callBaseFill_1 = require("../callBaseFill");
function StaticableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.isStatic = function () {
            return this.hasModifier(typescript_1.SyntaxKind.StaticKeyword);
        };
        class_1.prototype.getStaticKeyword = function () {
            return this.getFirstModifierByKind(typescript_1.SyntaxKind.StaticKeyword);
        };
        class_1.prototype.getStaticKeywordOrThrow = function () {
            return errors.throwIfNullOrUndefined(this.getStaticKeyword(), "Expected to find a static keyword.");
        };
        class_1.prototype.setIsStatic = function (value) {
            this.toggleModifier("static", value);
            return this;
        };
        class_1.prototype.fill = function (structure) {
            callBaseFill_1.callBaseFill(Base.prototype, this, structure);
            if (structure.isStatic != null)
                this.setIsStatic(structure.isStatic);
            return this;
        };
        return class_1;
    }(Base));
}
exports.StaticableNode = StaticableNode;
