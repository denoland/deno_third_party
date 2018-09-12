"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var manipulation_1 = require("../../manipulation");
function ChildOrderableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.setOrder = function (order) {
            var childIndex = this.getChildIndex();
            var parent = this.getParentSyntaxList() || this.getParentSyntaxListOrThrow();
            errors.throwIfOutOfRange(order, [0, parent.getChildCount() - 1], "order");
            if (childIndex === order)
                return this;
            manipulation_1.changeChildOrder({
                parent: parent,
                getSiblingFormatting: manipulation_1.getGeneralFormatting,
                oldIndex: childIndex,
                newIndex: order
            });
            return this;
        };
        return class_1;
    }(Base));
}
exports.ChildOrderableNode = ChildOrderableNode;
