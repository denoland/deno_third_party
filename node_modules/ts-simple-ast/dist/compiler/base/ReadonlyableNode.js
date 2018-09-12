"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var typescript_1 = require("../../typescript");
var callBaseFill_1 = require("../callBaseFill");
function ReadonlyableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.isReadonly = function () {
            return this.getReadonlyKeyword() != null;
        };
        class_1.prototype.getReadonlyKeyword = function () {
            return this.getFirstModifierByKind(typescript_1.SyntaxKind.ReadonlyKeyword);
        };
        class_1.prototype.getReadonlyKeywordOrThrow = function () {
            return errors.throwIfNullOrUndefined(this.getReadonlyKeyword(), "Expected to find a readonly keyword.");
        };
        class_1.prototype.setIsReadonly = function (value) {
            this.toggleModifier("readonly", value);
            return this;
        };
        class_1.prototype.fill = function (structure) {
            callBaseFill_1.callBaseFill(Base.prototype, this, structure);
            if (structure.isReadonly != null)
                this.setIsReadonly(structure.isReadonly);
            return this;
        };
        return class_1;
    }(Base));
}
exports.ReadonlyableNode = ReadonlyableNode;
