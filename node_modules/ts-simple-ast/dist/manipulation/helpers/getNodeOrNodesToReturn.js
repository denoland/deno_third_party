"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
function getNodeOrNodesToReturn(nodes, index, length) {
    return length > 0 ? getNodesToReturn(nodes, index, length) : nodes[index];
}
exports.getNodeOrNodesToReturn = getNodeOrNodesToReturn;
function getNodesToReturn(nodes, index, length) {
    return nodes.slice(index, index + length);
}
exports.getNodesToReturn = getNodesToReturn;
